//! `QueryBuilder`: builds a DQL string from parts, the way
//! `Doctrine\DBAL\Query\QueryBuilder` builds SQL.
//!
//! The DQL parts are PHP's `$dqlParts` array, a `PhpArray` of
//! `php_value` values whose expression objects are handles, as in PHP
//! (`andWhere()` adds to the `Andx` that `getDQLPart('where')` returned).
//! The variadic methods take a `Vec`; Rust has no named arguments, so the
//! check of `NoUnknownNamedArguments` has no counterpart. `getQuery()`
//! gives a [`StandInQuery`] until the port of `Query.php` is merged.
//!
//! Upstream: doctrine/orm src/QueryBuilder.php

#![allow(clippy::doc_markdown, reason = "upstream's doc comments")]

use std::cell::RefCell;
use std::sync::Arc;

use php_value::numeric::is_numeric;
use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_collections::expr::ExpressionVisitor as _;
use php2rust_doctrine_dbal::WrapperParameterType;

use crate::deprecations;
use crate::query::context::{DqlQuery as _, Hint, QueryContext};
use crate::query::expr::{
    self, Andx, Comparison, Composite, From, Func, GroupBy, Join, Literal, Math, Order, OrderBy,
    Orx, Select,
};
use crate::query::{
    Expr, Parameter, ParameterName, QueryExpressionVisitor, SortDirection, StandInQuery,
    query_exception,
};
use crate::query_type::QueryType;

/// The DQL part names, in `$dqlParts`' order.
const DQL_PART_NAMES: [&str; 9] = [
    "distinct", "select", "from", "join", "set", "where", "groupBy", "having", "orderBy",
];

/// `string|Expr\OrderBy $sort`.
#[derive(Debug, Clone)]
pub enum Sort {
    Field(String),
    OrderBy(OrderBy),
}

impl std::convert::From<&str> for Sort {
    fn from(sort: &str) -> Self {
        Self::Field(sort.to_owned())
    }
}

impl std::convert::From<String> for Sort {
    fn from(sort: String) -> Self {
        Self::Field(sort)
    }
}

impl std::convert::From<OrderBy> for Sort {
    fn from(sort: OrderBy) -> Self {
        Self::OrderBy(sort)
    }
}

/// This class is responsible for building DQL query strings via an object oriented
/// PHP interface.
pub struct QueryBuilder {
    /// The EntityManager used by this QueryBuilder.
    em: Arc<dyn QueryContext>,
    /// The array of DQL parts collected.
    dql_parts: RefCell<PhpArray>,
    /// The type of query this is. Can be select, update or delete.
    type_: QueryType,
    /// The complete DQL string for this query.
    dql: RefCell<Option<String>>,
    /// The query parameters.
    parameters: Vec<Parameter>,
    /// The index of the first result to retrieve.
    first_result: i64,
    /// The maximum number of results to retrieve.
    max_results: Option<i64>,
    /// Keeps root entity alias names for join entities.
    join_root_aliases: Vec<(String, String)>,
    /// Whether to use second level cache, if available.
    cacheable: bool,
    /// Second level cache region name.
    cache_region: Option<String>,
    /// Second level query cache mode.
    cache_mode: Option<i64>,
    lifetime: i64,
    /// The counter of bound parameters.
    bound_counter: i64,
    /// Query hints.
    hints: Vec<(String, Hint)>,
}

impl std::fmt::Debug for QueryBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryBuilder")
            .field("dql_parts", &self.dql_parts.borrow())
            .field("type_", &self.type_)
            .field("parameters", &self.parameters)
            .finish_non_exhaustive()
    }
}

/// `clone $element` of an expression object; any other object keeps its
/// handle, as `php_value` objects cannot be copied.
fn clone_element(value: &Value) -> Value {
    let Value::Object(object) = value else {
        return value.clone();
    };
    macro_rules! clone_as {
        ($($class:ty),*) => {
            $(if let Some(e) = object.downcast_ref::<$class>() {
                return Value::from(e.clone());
            })*
        };
    }
    clone_as!(
        Andx, Orx, Composite, Select, GroupBy, Literal, Comparison, Func, Math, From, Join, OrderBy
    );
    value.clone()
}

/// `$value instanceof $class`.
fn instance_of(value: &Value, class: &str) -> bool {
    value.as_object().is_some_and(|object| {
        object.class().eq_ignore_ascii_case(class) || object.instance_of(class)
    })
}

/// `(string) $value`, as UTF-8.
fn string_of(value: &Value) -> Result<String, Thrown> {
    Ok(String::from_utf8_lossy(expr::to_string(value)?.as_bytes()).into_owned())
}

/// `implode($separator, $array)`.
fn implode(separator: &str, array: &PhpArray) -> Result<String, Thrown> {
    let mut parts = Vec::with_capacity(array.len());
    for value in array.values() {
        parts.push(string_of(value)?);
    }
    Ok(parts.join(separator))
}

/// A PHP string is falsy when it is `""` or `"0"`.
fn truthy(text: &str) -> bool {
    !text.is_empty() && text != "0"
}

fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

fn runtime_exception(message: &str) -> Thrown {
    Thrown::new("RuntimeException", message, 0)
}

fn invalid_argument(message: String) -> Thrown {
    Thrown::new("InvalidArgumentException", message, 0)
}

impl QueryBuilder {
    pub const CLASS: &'static str = "Doctrine\\ORM\\QueryBuilder";

    /// Initializes a new `QueryBuilder` that uses the given `EntityManager`.
    pub fn new(em: Arc<dyn QueryContext>) -> Self {
        let mut dql_parts = PhpArray::new();
        for name in DQL_PART_NAMES {
            let initial = match name {
                "distinct" => Value::Bool(false),
                "where" | "having" => Value::Null,
                _ => Value::Array(PhpArray::new()),
            };
            dql_parts.insert(name, initial);
        }
        Self {
            em,
            dql_parts: RefCell::new(dql_parts),
            type_: QueryType::Select,
            dql: RefCell::new(None),
            parameters: Vec::new(),
            first_result: 0,
            max_results: None,
            join_root_aliases: Vec::new(),
            cacheable: false,
            cache_region: None,
            cache_mode: None,
            lifetime: 0,
            bound_counter: 0,
            hints: Vec::new(),
        }
    }

    /// `getType()` (`final protected` upstream).
    pub fn get_type(&self) -> QueryType {
        self.type_
    }

    /// Gets an ExpressionBuilder used for object-oriented construction of query expressions.
    ///
    /// Upstream asks the entity manager (`getExpressionBuilder()`); `Expr`
    /// has no state, so this is a new one.
    pub fn expr(&self) -> Expr {
        Expr::new()
    }

    /// Enable/disable second level query (result) caching for this query.
    pub fn set_cacheable(&mut self, cacheable: bool) -> &mut Self {
        self.cacheable = cacheable;
        self
    }

    /// Are the query results enabled for second level cache?
    pub fn is_cacheable(&self) -> bool {
        self.cacheable
    }

    pub fn set_cache_region(&mut self, cache_region: &str) -> &mut Self {
        self.cache_region = Some(cache_region.to_owned());
        self
    }

    /// Obtain the name of the second level query cache region in which query results will be stored
    pub fn get_cache_region(&self) -> Option<&str> {
        self.cache_region.as_deref()
    }

    pub fn get_lifetime(&self) -> i64 {
        self.lifetime
    }

    /// Sets the life-time for this query into second level cache.
    pub fn set_lifetime(&mut self, lifetime: i64) -> &mut Self {
        self.lifetime = lifetime;
        self
    }

    /// `getHints()`, in the order they were set.
    pub fn get_hints(&self) -> &[(String, Hint)] {
        &self.hints
    }

    /// `getHint($name)`: `false` for a hint that is not set or is `null`.
    pub fn get_hint(&self, name: &str) -> Hint {
        self.hints
            .iter()
            .find(|(n, h)| n == name && !matches!(h, Hint::Value(Value::Null)))
            .map_or(Hint::Value(Value::Bool(false)), |(_, h)| h.clone())
    }

    /// `hasHint($name)`: `isset($this->hints[$name])`.
    pub fn has_hint(&self, name: &str) -> bool {
        self.hints
            .iter()
            .any(|(n, h)| n == name && !matches!(h, Hint::Value(Value::Null)))
    }

    /// Adds a hint for the query.
    pub fn set_hint(&mut self, name: &str, value: Hint) -> &mut Self {
        match self.hints.iter_mut().find(|(n, _)| n == name) {
            Some((_, hint)) => *hint = value,
            None => self.hints.push((name.to_owned(), value)),
        }
        self
    }

    pub fn get_cache_mode(&self) -> Option<i64> {
        self.cache_mode
    }

    pub fn set_cache_mode(&mut self, cache_mode: i64) -> &mut Self {
        self.cache_mode = Some(cache_mode);
        self
    }

    /// Gets the associated EntityManager to this query builder.
    pub fn get_entity_manager(&self) -> &Arc<dyn QueryContext> {
        &self.em
    }

    /// Gets the complete DQL string formed by the current specifications of this QueryBuilder.
    pub fn get_dql(&self) -> Result<String, Thrown> {
        if let Some(dql) = self.dql.borrow().as_ref() {
            return Ok(dql.clone());
        }
        let dql = match self.type_ {
            QueryType::Select => self.get_dql_for_select()?,
            QueryType::Delete => self.get_dql_for_delete()?,
            QueryType::Update => self.get_dql_for_update()?,
        };
        *self.dql.borrow_mut() = Some(dql.clone());
        Ok(dql)
    }

    /// Constructs a Query instance from the current specifications of the builder.
    pub fn get_query(&self) -> Result<StandInQuery, Thrown> {
        let query = StandInQuery::new(Arc::clone(&self.em), &self.get_dql()?);
        query
            .set_parameters(self.parameters.clone())
            .set_first_result(Some(self.first_result))
            .set_max_results(self.max_results);

        if self.lifetime != 0 {
            query.set_lifetime(self.lifetime);
        }

        if let Some(cache_mode) = self.cache_mode.filter(|mode| *mode != 0) {
            query.set_cache_mode(Some(cache_mode));
        }

        if self.cacheable {
            query.set_cacheable(self.cacheable);
        }

        if let Some(cache_region) = self.cache_region.as_deref().filter(|r| truthy(r)) {
            query.set_cache_region(cache_region);
        }

        for (name, value) in &self.hints {
            query.set_hint(name, value.clone());
        }

        Ok(query)
    }

    /// Finds the root entity alias of the joined entity.
    fn find_root_alias(&mut self, alias: &str, parent_alias: &str) -> Result<String, Thrown> {
        let root_aliases = self.get_root_aliases()?;
        let root_alias = if root_aliases.iter().any(|a| a == parent_alias) {
            parent_alias.to_owned()
        } else if let Some((_, root)) = self
            .join_root_aliases
            .iter()
            .find(|(join, _)| join == parent_alias)
        {
            root.clone()
        } else {
            // Should never happen with correct joining order. Might be
            // thoughtful to throw exception instead.
            root_aliases.into_iter().next().ok_or_else(|| {
                runtime_exception("No alias was set before invoking getRootAlias().")
            })?
        };

        match self
            .join_root_aliases
            .iter_mut()
            .find(|(join, _)| join == alias)
        {
            Some((_, root)) => root.clone_from(&root_alias),
            None => self
                .join_root_aliases
                .push((alias.to_owned(), root_alias.clone())),
        }

        Ok(root_alias)
    }

    /// Gets the FIRST root alias of the query. This is the first entity alias involved
    /// in the construction of the query.
    pub fn get_root_alias(&self) -> Result<String, Thrown> {
        self.get_root_aliases()?
            .into_iter()
            .next()
            .ok_or_else(|| runtime_exception("No alias was set before invoking getRootAlias()."))
    }

    /// The `from` clauses, the ones given as a string turned into
    /// `Expr\From` objects (upstream does this in place, by reference).
    fn root_from_clauses(&self) -> Result<Vec<From>, Thrown> {
        let mut parts = self.dql_parts.borrow_mut();
        let Some(Value::Array(from_parts)) = parts.get_mut(&Key::from("from")) else {
            return Ok(Vec::new());
        };
        let keys: Vec<Key> = from_parts.keys().cloned().collect();
        let mut clauses = Vec::with_capacity(keys.len());
        for key in keys {
            let Some(from_clause) = from_parts.get_mut(&key) else {
                continue;
            };
            if let Value::String(text) = from_clause {
                let text = String::from_utf8_lossy(text.as_bytes()).into_owned();
                // `strrpos()` gives `false`, so `substr($from, 0, false)`
                // and `substr($from, 1)`, when there is no space.
                let (from, alias) = match text.rfind(' ') {
                    Some(pos) => (&text[..pos], &text[pos + 1..]),
                    None => ("", text.get(1..).unwrap_or("")),
                };
                *from_clause = Value::from(From::new(from, alias, None));
            }
            let clause = match from_clause {
                Value::Object(object) => object.downcast_ref::<From>().cloned(),
                _ => None,
            };
            match clause {
                Some(clause) => clauses.push(clause),
                None => {
                    return Err(Thrown::new(
                        "Error",
                        match from_clause {
                            Value::Object(object) => {
                                format!("Call to undefined method {}::getAlias()", object.class())
                            }
                            other => format!(
                                "Call to a member function getAlias() on {}",
                                other.debug_type()
                            ),
                        },
                        0,
                    ));
                }
            }
        }
        Ok(clauses)
    }

    /// Gets the root aliases of the query. This is the entity aliases involved
    /// in the construction of the query.
    pub fn get_root_aliases(&self) -> Result<Vec<String>, Thrown> {
        Ok(self
            .root_from_clauses()?
            .iter()
            .map(|from| from.get_alias().to_owned())
            .collect())
    }

    /// Gets all the aliases that have been used in the query.
    /// Including all select root aliases and join aliases
    pub fn get_all_aliases(&self) -> Result<Vec<String>, Thrown> {
        let mut aliases = self.get_root_aliases()?;
        aliases.extend(self.join_root_aliases.iter().map(|(join, _)| join.clone()));
        Ok(aliases)
    }

    /// Gets the root entities of the query. This is the entity classes involved
    /// in the construction of the query.
    pub fn get_root_entities(&self) -> Result<Vec<String>, Thrown> {
        Ok(self
            .root_from_clauses()?
            .iter()
            .map(|from| from.get_from().to_owned())
            .collect())
    }

    /// Sets a query parameter for the query being constructed.
    pub fn set_parameter(
        &mut self,
        key: impl Into<ParameterName>,
        value: impl Into<Value>,
        type_: Option<WrapperParameterType>,
    ) -> &mut Self {
        let key = key.into();
        let name = Parameter::normalize_name(key.clone());
        match self.parameters.iter_mut().find(|p| p.get_name() == name) {
            Some(existing) => existing.set_value(value, type_),
            None => self.parameters.push(Parameter::new(key, value, type_)),
        }
        self
    }

    /// Sets a collection of query parameters for the query being constructed.
    pub fn set_parameters(&mut self, parameters: Vec<Parameter>) -> &mut Self {
        self.parameters = parameters;
        self
    }

    /// Gets all defined query parameters for the query being constructed.
    pub fn get_parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Gets a (previously set) query parameter of the query being constructed.
    pub fn get_parameter(&self, key: impl Into<ParameterName>) -> Option<&Parameter> {
        let key = Parameter::normalize_name(key);
        self.parameters.iter().find(|p| p.get_name() == key)
    }

    /// Sets the position of the first result to retrieve (the "offset").
    pub fn set_first_result(&mut self, first_result: Option<i64>) -> &mut Self {
        self.first_result = first_result.unwrap_or(0);
        self
    }

    /// Gets the position of the first result the query object was set to retrieve (the "offset").
    pub fn get_first_result(&self) -> i64 {
        self.first_result
    }

    /// Sets the maximum number of results to retrieve (the "limit").
    pub fn set_max_results(&mut self, max_results: Option<i64>) -> Result<&mut Self, Thrown> {
        if matches!(self.type_, QueryType::Delete | QueryType::Update) {
            return Err(runtime_exception(
                "Setting a limit is not supported for delete or update queries.",
            ));
        }
        self.max_results = max_results;
        Ok(self)
    }

    /// Gets the maximum number of results the query object was set to retrieve (the "limit").
    /// Returns NULL if {@link setMaxResults} was not applied to this query builder.
    pub fn get_max_results(&self) -> Option<i64> {
        self.max_results
    }

    /// Either appends to or replaces a single, generic query part.
    ///
    /// The available parts are: 'select', 'from', 'join', 'set', 'where',
    /// 'groupBy', 'having' and 'orderBy'.
    pub fn add(
        &mut self,
        dql_part_name: &str,
        dql_part: impl Into<Value>,
        append: bool,
    ) -> Result<&mut Self, Thrown> {
        let mut dql_part = dql_part.into();
        if append && (dql_part_name == "where" || dql_part_name == "having") {
            return Err(invalid_argument(
                "Using $append = true does not have an effect with 'where' or 'having' parts. See QueryBuilder#andWhere() for an example for correct usage.".to_owned(),
            ));
        }

        let name_key = Key::from(dql_part_name);
        let is_multiple = matches!(
            self.dql_parts.borrow().get(&name_key),
            Some(Value::Array(_))
        ) && (dql_part_name != "join" || append);

        // Allow adding any part retrieved from self::getDQLParts().
        if dql_part_name != "join"
            && let Value::Array(array) = &dql_part
        {
            dql_part = array
                .first()
                .map_or(Value::Bool(false), |(_, first)| first.clone());
        }

        if dql_part_name == "join" {
            let entries: Vec<(Key, Value)> = match &dql_part {
                Value::Array(array) => array.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                Value::Object(object) => object
                    .properties()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                _ => Vec::new(),
            };
            let mut new_dql_part = PhpArray::new();
            for (k, v) in entries {
                let numeric = match &k {
                    Key::Int(_) => true,
                    Key::Str(s) => is_numeric(s.as_bytes()),
                };
                let k = if numeric {
                    deprecations::trigger("https://github.com/doctrine/orm/pull/12051", || {
                        "Using numeric keys in Doctrine\\ORM\\QueryBuilder::add for join parts is deprecated and will not be supported in 4.0. Use an associative array with the root alias as key instead.".to_owned()
                    });
                    let aliases = self.get_root_aliases()?;
                    let Some(first) = aliases.into_iter().next() else {
                        return Err(runtime_exception("No alias was set before invoking add()."));
                    };
                    Key::from(first.as_str())
                } else {
                    k
                };
                new_dql_part.insert(k, v);
            }
            dql_part = Value::Array(new_dql_part);
        }

        {
            let mut parts = self.dql_parts.borrow_mut();
            if append && is_multiple {
                let Some(Value::Array(target)) = parts.get_mut(&name_key) else {
                    return Err(runtime_exception("[] operator not supported"));
                };
                if let Value::Array(array) = &dql_part {
                    let (key, value) = array.first().map_or_else(
                        || (Key::from(""), Value::Null),
                        |(k, v)| (k.clone(), v.clone()),
                    );
                    if !target.contains_key(&key) {
                        target.insert(key.clone(), Value::Array(PhpArray::new()));
                    }
                    match target.get_mut(&key) {
                        Some(Value::Array(entry)) => {
                            push(entry, value)?;
                        }
                        Some(Value::Null) | None => {
                            target.insert(key, list(vec![value]));
                        }
                        Some(_) => {
                            return Err(Thrown::new(
                                "Error",
                                "[] operator not supported for strings",
                                0,
                            ));
                        }
                    }
                } else {
                    push(target, dql_part)?;
                }
            } else {
                let value = if is_multiple {
                    list(vec![dql_part])
                } else {
                    dql_part
                };
                parts.insert(name_key, value);
            }
        }

        *self.dql.borrow_mut() = None;

        Ok(self)
    }

    /// Specifies an item that is to be returned in the query result.
    /// Replaces any previously specified selections, if any.
    pub fn select(&mut self, select: Vec<Value>) -> Result<&mut Self, Thrown> {
        self.type_ = QueryType::Select;

        if select.is_empty() {
            return Ok(self);
        }

        let select = Select::new(list(select))?;
        self.add("select", select, false)
    }

    /// Adds a DISTINCT flag to this query.
    pub fn distinct(&mut self, flag: bool) -> &mut Self {
        let key = Key::from("distinct");
        let mut parts = self.dql_parts.borrow_mut();
        if !matches!(parts.get(&key), Some(Value::Bool(current)) if *current == flag) {
            parts.insert(key, Value::Bool(flag));
            *self.dql.borrow_mut() = None;
        }
        drop(parts);
        self
    }

    /// Adds an item that is to be returned in the query result.
    pub fn add_select(&mut self, select: Vec<Value>) -> Result<&mut Self, Thrown> {
        self.type_ = QueryType::Select;

        if select.is_empty() {
            return Ok(self);
        }

        let select = Select::new(list(select))?;
        self.add("select", select, true)
    }

    /// Turns the query being built into a bulk delete query that ranges over
    /// a certain entity type.
    pub fn delete(
        &mut self,
        delete: Option<&str>,
        alias: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        self.type_ = QueryType::Delete;

        let Some(delete) = delete.filter(|d| truthy(d)) else {
            return Ok(self);
        };

        let Some(alias) = alias.filter(|a| truthy(a)) else {
            return Err(invalid_argument(format!(
                "Doctrine\\ORM\\QueryBuilder::delete(): The alias for entity {delete} must not be omitted."
            )));
        };

        self.add("from", From::new(delete, alias, None), false)
    }

    /// Turns the query being built into a bulk update query that ranges over
    /// a certain entity type.
    pub fn update(
        &mut self,
        update: Option<&str>,
        alias: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        self.type_ = QueryType::Update;

        let Some(update) = update.filter(|u| truthy(u)) else {
            return Ok(self);
        };

        let Some(alias) = alias.filter(|a| truthy(a)) else {
            return Err(invalid_argument(format!(
                "Doctrine\\ORM\\QueryBuilder::update(): The alias for entity {update} must not be omitted."
            )));
        };

        self.add("from", From::new(update, alias, None), false)
    }

    /// Creates and adds a query root corresponding to the entity identified by the given alias,
    /// forming a cartesian product with any existing query roots.
    pub fn from(
        &mut self,
        from: &str,
        alias: &str,
        index_by: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        self.add("from", From::new(from, alias, index_by), true)
    }

    /// Updates a query root corresponding to an entity setting its index by. This method is intended to be used with
    /// EntityRepository->createQueryBuilder(), which creates the initial FROM clause and do not allow you to update it
    /// setting an index by.
    pub fn index_by(&mut self, alias: &str, index_by: &str) -> Result<&mut Self, Thrown> {
        let root_aliases = self.get_root_aliases()?;

        if !root_aliases.iter().any(|a| a == alias) {
            return Err(query_exception::new(format!(
                "Specified root alias {alias} must be set before invoking indexBy()."
            )));
        }

        {
            let mut parts = self.dql_parts.borrow_mut();
            if let Some(Value::Array(from_parts)) = parts.get_mut(&Key::from("from")) {
                let keys: Vec<Key> = from_parts.keys().cloned().collect();
                for key in keys {
                    let Some(from_clause) = from_parts.get_mut(&key) else {
                        continue;
                    };
                    let replacement = match from_clause {
                        Value::Object(object) => object
                            .downcast_ref::<From>()
                            .filter(|from| from.get_alias() == alias)
                            .map(|from| {
                                From::new(from.get_from(), from.get_alias(), Some(index_by))
                            }),
                        _ => None,
                    };
                    if let Some(replacement) = replacement {
                        *from_clause = Value::from(replacement);
                    }
                }
            }
        }

        Ok(self)
    }

    /// Creates and adds a join over an entity association to the query
    /// (an inner join).
    pub fn join(
        &mut self,
        join: &str,
        alias: &str,
        condition_type: Option<&str>,
        condition: Option<Value>,
        index_by: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        self.inner_join(join, alias, condition_type, condition, index_by)
    }

    /// Creates and adds a join over an entity association to the query.
    pub fn inner_join(
        &mut self,
        join: &str,
        alias: &str,
        condition_type: Option<&str>,
        condition: Option<Value>,
        index_by: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        self.add_join(
            Join::INNER_JOIN,
            join,
            alias,
            condition_type,
            condition,
            index_by,
        )
    }

    /// Creates and adds a left join over an entity association to the query.
    pub fn left_join(
        &mut self,
        join: &str,
        alias: &str,
        condition_type: Option<&str>,
        condition: Option<Value>,
        index_by: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        self.add_join(
            Join::LEFT_JOIN,
            join,
            alias,
            condition_type,
            condition,
            index_by,
        )
    }

    /// `innerJoin()` and `leftJoin()`.
    fn add_join(
        &mut self,
        join_type: &str,
        join: &str,
        alias: &str,
        condition_type: Option<&str>,
        condition: Option<Value>,
        index_by: Option<&str>,
    ) -> Result<&mut Self, Thrown> {
        // `substr($join, 0, (int) strpos($join, '.'))`.
        let parent_alias = &join[..join.find('.').unwrap_or(0)];

        let root_alias = self.find_root_alias(alias, parent_alias)?;
        let join = Join::new(
            join_type,
            join,
            Some(alias),
            condition_type,
            condition.unwrap_or(Value::Null),
            index_by,
        );

        let mut part = PhpArray::new();
        part.insert(root_alias.as_str(), Value::from(join));
        self.add("join", Value::Array(part), true)
    }

    /// Sets a new value for a field in a bulk update query.
    pub fn set(&mut self, key: &str, value: impl Into<Value>) -> Result<&mut Self, Thrown> {
        self.add("set", Comparison::new(key, Comparison::EQ, value), true)
    }

    /// Specifies one or more restrictions to the query result.
    /// Replaces any previously specified restrictions, if any.
    pub fn r#where(&mut self, predicates: Vec<Value>) -> Result<&mut Self, Thrown> {
        let predicates = if predicates.len() == 1 && instance_of(&predicates[0], Composite::CLASS) {
            predicates.into_iter().next().unwrap_or(Value::Null)
        } else {
            Andx::new(list(predicates))?.into()
        };

        self.add("where", predicates, false)
    }

    /// Adds one or more restrictions to the query results, forming a logical
    /// conjunction with any previously specified restrictions.
    pub fn and_where(&mut self, mut r#where: Vec<Value>) -> Result<&mut Self, Thrown> {
        let dql = self.get_dql_part("where");

        let dql = if let Some(andx) = dql.as_object().and_then(|o| o.downcast_ref::<Andx>()) {
            andx.add_multiple(list(r#where))?;
            dql
        } else {
            r#where.insert(0, dql);
            Andx::new(list(r#where))?.into()
        };

        self.add("where", dql, false)
    }

    /// Adds one or more restrictions to the query results, forming a logical
    /// disjunction with any previously specified restrictions.
    pub fn or_where(&mut self, mut r#where: Vec<Value>) -> Result<&mut Self, Thrown> {
        let dql = self.get_dql_part("where");

        let dql = if let Some(orx) = dql.as_object().and_then(|o| o.downcast_ref::<Orx>()) {
            orx.add_multiple(list(r#where))?;
            dql
        } else {
            r#where.insert(0, dql);
            Orx::new(list(r#where))?.into()
        };

        self.add("where", dql, false)
    }

    /// Specifies a grouping over the results of the query.
    /// Replaces any previously specified groupings, if any.
    pub fn group_by(&mut self, group_by: &[&str]) -> Result<&mut Self, Thrown> {
        let group_by = GroupBy::new(list(group_by.iter().map(|g| Value::from(*g)).collect()))?;
        self.add("groupBy", group_by, false)
    }

    /// Adds a grouping expression to the query.
    pub fn add_group_by(&mut self, group_by: &[&str]) -> Result<&mut Self, Thrown> {
        let group_by = GroupBy::new(list(group_by.iter().map(|g| Value::from(*g)).collect()))?;
        self.add("groupBy", group_by, true)
    }

    /// Specifies a restriction over the groups of the query.
    /// Replaces any previous having restrictions, if any.
    pub fn having(&mut self, having: Vec<Value>) -> Result<&mut Self, Thrown> {
        let having = if having.len() == 1
            && (instance_of(&having[0], Andx::CLASS) || instance_of(&having[0], Orx::CLASS))
        {
            having.into_iter().next().unwrap_or(Value::Null)
        } else {
            Andx::new(list(having))?.into()
        };

        self.add("having", having, false)
    }

    /// Adds a restriction over the groups of the query, forming a logical
    /// conjunction with any existing having restrictions.
    pub fn and_having(&mut self, mut having: Vec<Value>) -> Result<&mut Self, Thrown> {
        let dql = self.get_dql_part("having");

        let dql = if let Some(andx) = dql.as_object().and_then(|o| o.downcast_ref::<Andx>()) {
            andx.add_multiple(list(having))?;
            dql
        } else {
            having.insert(0, dql);
            Andx::new(list(having))?.into()
        };

        self.add("having", dql, false)
    }

    /// Adds a restriction over the groups of the query, forming a logical
    /// disjunction with any existing having restrictions.
    pub fn or_having(&mut self, mut having: Vec<Value>) -> Result<&mut Self, Thrown> {
        let dql = self.get_dql_part("having");

        let dql = if let Some(orx) = dql.as_object().and_then(|o| o.downcast_ref::<Orx>()) {
            orx.add_multiple(list(having))?;
            dql
        } else {
            having.insert(0, dql);
            Orx::new(list(having))?.into()
        };

        self.add("having", dql, false)
    }

    /// Specifies an ordering for the query results.
    /// Replaces any previously specified orderings, if any.
    ///
    /// `order` is `None` for a call without `$order` (upstream's
    /// `func_num_args() === 1`), `Some(Order::Null)` for an explicit `null`.
    pub fn order_by(
        &mut self,
        sort: impl Into<Sort>,
        order: Option<Order>,
    ) -> Result<&mut Self, Thrown> {
        let order_by = Self::order_by_expression(sort.into(), order, "orderBy")?;
        self.add("orderBy", order_by, false)
    }

    /// Adds an ordering to the query results.
    pub fn add_order_by(
        &mut self,
        sort: impl Into<Sort>,
        order: Option<Order>,
    ) -> Result<&mut Self, Thrown> {
        let order_by = Self::order_by_expression(sort.into(), order, "addOrderBy")?;
        self.add("orderBy", order_by, true)
    }

    fn order_by_expression(
        sort: Sort,
        order: Option<Order>,
        method: &str,
    ) -> Result<OrderBy, Thrown> {
        match sort {
            Sort::OrderBy(order_by) => Ok(order_by),
            Sort::Field(sort) => {
                let order = order.unwrap_or(Order::Direction(SortDirection::Ascending));
                let direction = Self::get_sort_direction(&order, method)?;
                Ok(OrderBy::new(Some(&sort), direction))
            }
        }
    }

    /// Adds criteria to the query.
    ///
    /// Adds where expressions with AND operator.
    /// Adds orderings.
    /// Overrides firstResult and maxResults if they're set.
    pub fn add_criteria(&mut self, criteria: &Criteria) -> Result<&mut Self, Thrown> {
        let all_aliases = self.get_all_aliases()?;
        let Some(first_alias) = all_aliases.first().cloned() else {
            return Err(query_exception::new(
                "No aliases are set before invoking addCriteria().",
            ));
        };

        let mut visitor = QueryExpressionVisitor::new(self.get_all_aliases()?);

        if let Some(where_expression) = criteria.get_where_expression() {
            let expression = visitor.dispatch(where_expression)?;
            self.and_where(vec![expression])?;
            self.parameters.extend(visitor.get_parameters());
        }

        for (sort, order) in criteria.get_orderings() {
            let has_valid_alias = all_aliases
                .iter()
                .any(|alias| format!("{sort}.").starts_with(&format!("{alias}.")));

            let sort = if has_valid_alias {
                sort
            } else {
                format!("{first_alias}.{sort}")
            };

            let order = match order {
                php2rust_doctrine_collections::SortDirection::Ascending => SortDirection::Ascending,
                php2rust_doctrine_collections::SortDirection::Descending => {
                    SortDirection::Descending
                }
            };
            self.add_order_by(sort, Some(Order::Direction(order)))?;
        }

        // Overwrite limits only if they was set in criteria
        let first_result = criteria.get_first_result();
        if first_result.is_some_and(|first| first > 0) {
            self.set_first_result(first_result);
        }

        let max_results = criteria.get_max_results();
        if max_results.is_some() {
            self.set_max_results(max_results)?;
        }

        Ok(self)
    }

    /// Gets a query part by its name (`null` for an unknown one).
    pub fn get_dql_part(&self, query_part_name: &str) -> Value {
        self.dql_parts
            .borrow()
            .get(&Key::from(query_part_name))
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// Gets all query parts.
    pub fn get_dql_parts(&self) -> PhpArray {
        self.dql_parts.borrow().clone()
    }

    fn get_dql_for_delete(&self) -> Result<String, Thrown> {
        Ok(format!(
            "DELETE{}{}{}",
            self.get_reduced_dql_query_part("from", " ", ", ")?,
            self.get_reduced_dql_query_part("where", " WHERE ", "")?,
            self.get_reduced_dql_query_part("orderBy", " ORDER BY ", ", ")?,
        ))
    }

    fn get_dql_for_update(&self) -> Result<String, Thrown> {
        Ok(format!(
            "UPDATE{}{}{}{}",
            self.get_reduced_dql_query_part("from", " ", ", ")?,
            self.get_reduced_dql_query_part("set", " SET ", ", ")?,
            self.get_reduced_dql_query_part("where", " WHERE ", "")?,
            self.get_reduced_dql_query_part("orderBy", " ORDER BY ", ", ")?,
        ))
    }

    fn get_dql_for_select(&self) -> Result<String, Thrown> {
        let mut dql = String::from("SELECT");
        if matches!(self.get_dql_part("distinct"), Value::Bool(true)) {
            dql.push_str(" DISTINCT");
        }
        dql.push_str(&self.get_reduced_dql_query_part("select", " ", ", ")?);

        let from_parts = self.get_dql_part("from");
        let join_parts = self.get_dql_part("join");
        let mut from_clauses = Vec::new();

        // Loop through all FROM clauses
        if from_parts.is_truthy() {
            dql.push_str(" FROM ");

            if let Value::Array(from_parts) = &from_parts {
                for from in from_parts.values() {
                    let mut from_clause = string_of(from)?;
                    let alias = from
                        .as_object()
                        .and_then(|o| o.downcast_ref::<From>())
                        .map(|f| f.get_alias().to_owned());
                    if let (Some(alias), Value::Array(join_parts)) = (alias, &join_parts)
                        && let Some(Value::Array(joins)) =
                            join_parts.get(&Key::from(alias.as_str()))
                    {
                        for join in joins.values() {
                            from_clause.push(' ');
                            from_clause.push_str(&string_of(join)?);
                        }
                    }
                    from_clauses.push(from_clause);
                }
            }
        }

        dql.push_str(&from_clauses.join(", "));
        dql.push_str(&self.get_reduced_dql_query_part("where", " WHERE ", "")?);
        dql.push_str(&self.get_reduced_dql_query_part("groupBy", " GROUP BY ", ", ")?);
        dql.push_str(&self.get_reduced_dql_query_part("having", " HAVING ", "")?);
        dql.push_str(&self.get_reduced_dql_query_part("orderBy", " ORDER BY ", ", ")?);

        Ok(dql)
    }

    fn get_reduced_dql_query_part(
        &self,
        query_part_name: &str,
        pre: &str,
        separator: &str,
    ) -> Result<String, Thrown> {
        let query_part = self.get_dql_part(query_part_name);

        if !query_part.is_truthy() {
            return Ok(String::new());
        }

        let body = match &query_part {
            Value::Array(array) => implode(separator, array)?,
            other => string_of(other)?,
        };
        Ok(format!("{pre}{body}"))
    }

    /// Resets DQL parts (all of them for `None`).
    pub fn reset_dql_parts(&mut self, parts: Option<&[&str]>) -> &mut Self {
        let parts: Vec<String> = match parts {
            Some(parts) => parts.iter().map(|p| (*p).to_owned()).collect(),
            None => self
                .dql_parts
                .borrow()
                .keys()
                .map(|k| String::from_utf8_lossy(&k.bytes()).into_owned())
                .collect(),
        };

        for part in parts {
            self.reset_dql_part(&part);
        }

        self
    }

    /// Resets single DQL part.
    pub fn reset_dql_part(&mut self, part: &str) -> &mut Self {
        let key = Key::from(part);
        let mut parts = self.dql_parts.borrow_mut();
        let reset = if matches!(parts.get(&key), Some(Value::Array(_))) {
            Value::Array(PhpArray::new())
        } else {
            Value::Null
        };
        parts.insert(key, reset);
        drop(parts);
        *self.dql.borrow_mut() = None;

        self
    }

    /// Creates a new named parameter and bind the value $value to it.
    ///
    /// The parameter $value specifies the value that you want to bind. If
    /// $placeholder is not provided createNamedParameter() will automatically
    /// create a placeholder for you. An automatic placeholder will be of the
    /// name ':dcValue1', ':dcValue2' etc.
    pub fn create_named_parameter(
        &mut self,
        value: impl Into<Value>,
        type_: Option<WrapperParameterType>,
        placeholder: Option<&str>,
    ) -> String {
        let placeholder = if let Some(placeholder) = placeholder {
            placeholder.to_owned()
        } else {
            self.bound_counter += 1;
            format!(":dcValue{}", self.bound_counter)
        };

        // `substr($placeholder, 1)`.
        let name = placeholder.get(1..).unwrap_or("").to_owned();
        self.set_parameter(name, value, type_);

        placeholder
    }

    /// Gets a string representation of this QueryBuilder which corresponds to
    /// the final DQL query being constructed.
    pub fn try_to_string(&self) -> Result<String, Thrown> {
        self.get_dql()
    }

    /// Converts the builder to the value PHP passes where an expression
    /// takes a `QueryBuilder` (`Expr::exists($qb)` and the like): a
    /// `Stringable` object whose string is the builder's DQL at the time of
    /// the conversion.
    pub fn to_value(&self) -> Result<Value, Thrown> {
        Ok(Value::Object(Arc::new(QueryBuilderDql {
            dql: self.get_dql()?,
        })))
    }

    /// `getSortDirection($order, $method)`.
    fn get_sort_direction(order: &Order, method: &str) -> Result<SortDirection, Thrown> {
        let debug_type = match order {
            Order::Direction(direction) => return Ok(*direction),
            Order::Name(_) => "string",
            Order::Null => "null",
        };

        deprecations::trigger(expr::ORDER_DEPRECATION, || {
            format!(
                "Passing {debug_type} as $order to Doctrine\\ORM\\QueryBuilder::{method}() is deprecated, use an instance of SortDirection instead."
            )
        });

        match order {
            Order::Name(name) => match name.to_lowercase().as_str() {
                "asc" => Ok(SortDirection::Ascending),
                "desc" => Ok(SortDirection::Descending),
                _ => Err(invalid_argument(format!(
                    "Invalid sort direction \"{name}\"."
                ))),
            },
            Order::Direction(_) | Order::Null => Ok(SortDirection::Ascending),
        }
    }
}

/// `$array[] = $value`.
fn push(array: &mut PhpArray, value: Value) -> Result<(), Thrown> {
    array.push(value).map(|_| ()).map_err(|_| {
        Thrown::new(
            "Error",
            "Cannot add element to the array as the next element is already occupied",
            0,
        )
    })
}

/// Deep clones the DQL parts and the parameters (`__clone()`).
impl Clone for QueryBuilder {
    fn clone(&self) -> Self {
        let mut dql_parts = self.dql_parts.borrow().clone();
        let keys: Vec<Key> = dql_parts.keys().cloned().collect();
        for key in keys {
            let Some(elements) = dql_parts.get_mut(&key) else {
                continue;
            };
            match elements {
                Value::Array(array) => {
                    let idxs: Vec<Key> = array.keys().cloned().collect();
                    for idx in idxs {
                        if let Some(element) = array.get_mut(&idx)
                            && matches!(element, Value::Object(_))
                        {
                            *element = clone_element(element);
                        }
                    }
                }
                Value::Object(_) => *elements = clone_element(elements),
                _ => {}
            }
        }

        Self {
            em: Arc::clone(&self.em),
            dql_parts: RefCell::new(dql_parts),
            type_: self.type_,
            dql: RefCell::new(self.dql.borrow().clone()),
            parameters: self.parameters.clone(),
            first_result: self.first_result,
            max_results: self.max_results,
            join_root_aliases: self.join_root_aliases.clone(),
            cacheable: self.cacheable,
            cache_region: self.cache_region.clone(),
            cache_mode: self.cache_mode,
            lifetime: self.lifetime,
            bound_counter: self.bound_counter,
            hints: self.hints.clone(),
        }
    }
}

/// A `QueryBuilder` as an expression argument: its DQL, as `__toString()`
/// gives it.
#[derive(Debug, Clone)]
struct QueryBuilderDql {
    dql: String,
}

impl php_value::PhpObject for QueryBuilderDql {
    fn class(&self) -> &str {
        QueryBuilder::CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn to_php_string(&self) -> Option<php_value::PhpString> {
        Some(php_value::PhpString::from(self.dql.as_str()))
    }

    fn is_stringable(&self) -> bool {
        true
    }

    fn instance_of(&self, class: &str) -> bool {
        class.eq_ignore_ascii_case("Stringable")
    }
}
