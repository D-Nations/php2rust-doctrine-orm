//! `Doctrine\ORM\Query\ParserResult`: the result of parsing a DQL query,
//! the SQL finalizer (or legacy executor), the result set mapping and the
//! parameter mappings.
//!
//! [`ResultSetMapping`] here is the part of
//! `Doctrine\ORM\Query\ResultSetMapping` that the SQL walker fills (its
//! public arrays and the `add*()` methods the walker calls), a stand-in
//! until the full class is joined in.
//!
//! Upstream: doctrine/orm src/Query/ParserResult.php

use php_value::{Key, PhpArray, Thrown, Value};

use crate::deprecations;
use crate::query::context::DqlQuery;
use crate::query::exec::{SqlExecutor, SqlFinalizer};
use crate::query::query_exception;

const SQL_EXECUTOR_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/11188";

fn s(value: &str) -> Value {
    Value::from(value)
}

fn array(value: Option<&Value>) -> PhpArray {
    match value {
        Some(Value::Array(a)) => a.clone(),
        _ => PhpArray::new(),
    }
}

/// `$array[$k1][$k2] = $value`.
fn set2(target: &mut PhpArray, k1: Key, k2: Key, value: Value) {
    let mut inner = array(target.get(&k1));
    inner.insert(k2, value);
    target.insert(k1, Value::Array(inner));
}

/// The part of `ResultSetMapping` the SQL walker fills.
#[derive(Debug, Clone, PartialEq)]
pub struct ResultSetMapping {
    pub is_mixed: bool,
    pub is_select: bool,
    pub alias_map: PhpArray,
    pub partial_aliases: PhpArray,
    pub relation_map: PhpArray,
    pub parent_alias_map: PhpArray,
    pub field_mappings: PhpArray,
    pub column_alias_mappings: PhpArray,
    pub scalar_mappings: PhpArray,
    pub enum_mappings: PhpArray,
    pub type_mappings: PhpArray,
    pub entity_mappings: PhpArray,
    pub meta_mappings: PhpArray,
    pub column_owner_map: PhpArray,
    pub discriminator_columns: PhpArray,
    pub index_by_map: PhpArray,
    pub declaring_classes: PhpArray,
    pub is_identifier_column: PhpArray,
    pub new_object_mappings: PhpArray,
    pub new_object: PhpArray,
    pub nested_new_object_arguments: PhpArray,
    pub metadata_parameter_mapping: PhpArray,
    pub discriminator_parameters: PhpArray,
    pub nested_entities: PhpArray,
}

impl Default for ResultSetMapping {
    fn default() -> Self {
        Self {
            is_mixed: false,
            is_select: true,
            alias_map: PhpArray::new(),
            partial_aliases: PhpArray::new(),
            relation_map: PhpArray::new(),
            parent_alias_map: PhpArray::new(),
            field_mappings: PhpArray::new(),
            column_alias_mappings: PhpArray::new(),
            scalar_mappings: PhpArray::new(),
            enum_mappings: PhpArray::new(),
            type_mappings: PhpArray::new(),
            entity_mappings: PhpArray::new(),
            meta_mappings: PhpArray::new(),
            column_owner_map: PhpArray::new(),
            discriminator_columns: PhpArray::new(),
            index_by_map: PhpArray::new(),
            declaring_classes: PhpArray::new(),
            is_identifier_column: PhpArray::new(),
            new_object_mappings: PhpArray::new(),
            new_object: PhpArray::new(),
            nested_new_object_arguments: PhpArray::new(),
            metadata_parameter_mapping: PhpArray::new(),
            discriminator_parameters: PhpArray::new(),
            nested_entities: PhpArray::new(),
        }
    }
}

impl ResultSetMapping {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entity result to this `ResultSetMapping`.
    pub fn add_entity_result(&mut self, class: &str, alias: &str, result_alias: Option<&str>) {
        self.alias_map.insert(alias, s(class));
        self.entity_mappings
            .insert(alias, result_alias.map_or(Value::Null, s));
        if result_alias.is_some() {
            self.is_mixed = true;
        }
    }

    pub fn mark_partial_entity_result(&mut self, alias: &str) {
        self.partial_aliases.insert(alias, Value::Bool(true));
    }

    /// Sets a discriminator column for an entity result or joined entity
    /// result.
    pub fn set_discriminator_column(&mut self, alias: &str, discr_column: &str) {
        self.discriminator_columns.insert(alias, s(discr_column));
        self.column_owner_map.insert(discr_column, s(alias));
    }

    /// Sets a field to use for indexing an entity result or joined entity
    /// result.
    pub fn add_index_by(&mut self, alias: &str, field_name: &str) {
        let mut columns = self.meta_mappings.clone();
        for (key, value) in self.field_mappings.iter() {
            columns.insert(key.clone(), value.clone());
        }
        for (column_name, column_field_name) in columns.iter() {
            let owner = self.column_owner_map.get(column_name);
            if !(column_field_name == &s(field_name) && owner == Some(&s(alias))) {
                continue;
            }
            let column = String::from_utf8_lossy(&column_name.bytes()).into_owned();
            self.add_index_by_column(alias, &column);
            break;
        }
    }

    /// Sets to index by a scalar result column name.
    pub fn add_index_by_scalar(&mut self, result_column_name: &str) {
        self.index_by_map.insert("scalars", s(result_column_name));
    }

    /// Sets a column to use for indexing an entity or joined entity
    /// result by the given alias name.
    pub fn add_index_by_column(&mut self, alias: &str, result_column_name: &str) {
        self.index_by_map.insert(alias, s(result_column_name));
    }

    /// Adds a field to the result that belongs to an entity or joined
    /// entity.
    pub fn add_field_result(
        &mut self,
        alias: &str,
        column_name: &str,
        field_name: &str,
        declaring_class: Option<&str>,
    ) {
        self.field_mappings.insert(column_name, s(field_name));
        self.column_owner_map.insert(column_name, s(alias));
        let declaring_class = match declaring_class {
            Some(class) if !class.is_empty() => s(class),
            _ => self
                .alias_map
                .get(&Key::from(alias))
                .cloned()
                .unwrap_or(Value::Null),
        };
        self.declaring_classes
            .insert(column_name, declaring_class.clone());
        let class_key = Key::from_value(&declaring_class).unwrap_or(Key::from(""));
        let mut by_alias = array(self.column_alias_mappings.get(&class_key));
        set2(
            &mut by_alias,
            Key::from(alias),
            Key::from(field_name),
            s(column_name),
        );
        self.column_alias_mappings
            .insert(class_key, Value::Array(by_alias));
        if !self.is_mixed && !self.scalar_mappings.is_empty() {
            self.is_mixed = true;
        }
    }

    /// Adds a joined entity result.
    pub fn add_joined_entity_result(
        &mut self,
        class: &str,
        alias: &str,
        parent_alias: &str,
        relation: &str,
        is_partial: bool,
    ) {
        self.alias_map.insert(alias, s(class));
        self.parent_alias_map.insert(alias, s(parent_alias));
        self.relation_map.insert(alias, s(relation));
        if is_partial {
            self.partial_aliases.insert(alias, Value::Bool(true));
        }
    }

    /// Adds a scalar result mapping.
    pub fn add_scalar_result(&mut self, column_name: &str, alias: &Key, type_: &str) {
        self.scalar_mappings.insert(column_name, alias.to_value());
        self.type_mappings.insert(column_name, s(type_));
        if !self.is_mixed && !self.field_mappings.is_empty() {
            self.is_mixed = true;
        }
    }

    /// Adds a scalar result mapping.
    pub fn add_enum_result(&mut self, column_name: &str, enum_type: &str) {
        self.enum_mappings.insert(column_name, s(enum_type));
    }

    /// Adds a meta column (foreign key or discriminator column) to the
    /// result set.
    pub fn add_meta_result(
        &mut self,
        alias: &str,
        column_name: &str,
        field_name: &str,
        is_identifier_column: bool,
        type_: Option<&str>,
    ) {
        self.meta_mappings.insert(column_name, s(field_name));
        self.column_owner_map.insert(column_name, s(alias));
        if is_identifier_column {
            set2(
                &mut self.is_identifier_column,
                Key::from(alias),
                Key::from(column_name),
                Value::Bool(true),
            );
        }
        if let Some(type_) = type_.filter(|t| !t.is_empty() && *t != "0") {
            self.type_mappings.insert(column_name, s(type_));
        }
    }
}

/// `ParserResult`.
#[derive(Default)]
pub struct ParserResult {
    sql_executor: Option<SqlExecutor>,
    sql_finalizer: Option<Box<dyn SqlFinalizer>>,
    result_set_mapping: ResultSetMapping,
    /// The mappings of DQL parameter names/positions to SQL parameter
    /// positions.
    parameter_mappings: PhpArray,
}

impl std::fmt::Debug for ParserResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParserResult")
            .field("sql_executor", &self.sql_executor)
            .field("result_set_mapping", &self.result_set_mapping)
            .field("parameter_mappings", &self.parameter_mappings)
            .finish_non_exhaustive()
    }
}

impl ParserResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the `ResultSetMapping` for the parsed query.
    pub fn get_result_set_mapping(&self) -> &ResultSetMapping {
        &self.result_set_mapping
    }

    /// The `ResultSetMapping`, to fill it.
    pub fn get_result_set_mapping_mut(&mut self) -> &mut ResultSetMapping {
        &mut self.result_set_mapping
    }

    /// Sets the `ResultSetMapping` of the parsed query.
    pub fn set_result_set_mapping(&mut self, rsm: ResultSetMapping) {
        self.result_set_mapping = rsm;
    }

    /// Sets the SQL executor that should be used for this `ParserResult`.
    pub fn set_sql_executor(&mut self, executor: SqlExecutor) {
        deprecations::trigger(SQL_EXECUTOR_DEPRECATION, || {
            "The SqlExecutor will be removed from Doctrine\\ORM\\Query\\ParserResult in 4.0. Provide a Doctrine\\ORM\\Query\\Exec\\SqlFinalizer instead that can create the executor.".to_owned()
        });
        self.sql_executor = Some(executor);
    }

    /// Gets the SQL executor used by this `ParserResult`.
    pub fn get_sql_executor(&self) -> Result<&SqlExecutor, Thrown> {
        deprecations::trigger(SQL_EXECUTOR_DEPRECATION, || {
            "The SqlExecutor will be removed from Doctrine\\ORM\\Query\\ParserResult in 4.0. Provide a Doctrine\\ORM\\Query\\Exec\\SqlFinalizer instead that can create the executor.".to_owned()
        });
        self.sql_executor.as_ref().ok_or_else(|| {
            query_exception::logic(
                "Executor not set yet. Call Doctrine\\ORM\\Query\\ParserResult::setSqlExecutor() first.",
            )
        })
    }

    pub fn set_sql_finalizer(&mut self, finalizer: Box<dyn SqlFinalizer>) {
        self.sql_finalizer = Some(finalizer);
    }

    /// The SQL finalizer, if one is set.
    pub fn get_sql_finalizer(&self) -> Option<&dyn SqlFinalizer> {
        self.sql_finalizer.as_deref()
    }

    pub fn prepare_sql_executor(&self, query: &dyn DqlQuery) -> Result<SqlExecutor, Thrown> {
        if let Some(finalizer) = &self.sql_finalizer {
            return finalizer.create_executor(query);
        }
        if let Some(executor) = &self.sql_executor {
            return Ok(executor.clone());
        }
        Err(query_exception::logic(
            "This ParserResult lacks both the SqlFinalizer as well as the (legacy) SqlExecutor",
        ))
    }

    /// Adds a DQL to SQL parameter mapping. One DQL parameter name/position
    /// can map to several SQL parameter positions.
    pub fn add_parameter_mapping(&mut self, dql_position: Key, sql_position: i64) {
        let mut positions = array(self.parameter_mappings.get(&dql_position));
        // A list: the next index is always free.
        let _ = positions.push(Value::Int(sql_position));
        self.parameter_mappings
            .insert(dql_position, Value::Array(positions));
    }

    /// Gets all DQL to SQL parameter mappings.
    pub fn get_parameter_mappings(&self) -> &PhpArray {
        &self.parameter_mappings
    }

    /// Gets the SQL parameter positions for a DQL parameter name/position.
    pub fn get_sql_parameter_positions(&self, dql_position: &Key) -> Option<Vec<i64>> {
        match self.parameter_mappings.get(dql_position) {
            Some(Value::Array(positions)) => Some(
                positions
                    .values()
                    .filter_map(|v| match v {
                        Value::Int(i) => Some(*i),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        }
    }
}
