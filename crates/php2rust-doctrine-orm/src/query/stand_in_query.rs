//! A stand-in for `Doctrine\ORM\Query` while the port of `Query.php` and
//! `AbstractQuery.php` is not merged: the state `QueryBuilder::getQuery()`
//! and the paginator set and read (the DQL, the parameters, the first
//! and max results, the hints, the hydration mode and the second-level
//! cache options) and `getSQL()`, which parses the DQL. It neither
//! executes nor hydrates.
//!
//! Upstream: none

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use php_value::{Thrown, Value};
use php2rust_doctrine_dbal::WrapperParameterType;

use crate::query::context::{DqlQuery, HYDRATE_OBJECT, Hint, QueryContext};
use crate::query::exec::SqlStatements;
use crate::query::result_set_mapping::ResultSetMapping;
use crate::query::{Parameter, ParameterName, Parser, ParserResult};

/// `$query` of `EntityManager::createQuery($dql)`, as far as the query
/// builder and the paginator use it.
pub struct StandInQuery {
    em: Arc<dyn QueryContext>,
    dql: RefCell<String>,
    parameters: RefCell<Vec<Parameter>>,
    hints: RefCell<Vec<(String, Hint)>>,
    hydration_mode: Cell<i64>,
    first_result: Cell<i64>,
    max_results: Cell<Option<i64>>,
    cacheable: Cell<bool>,
    lifetime: Cell<i64>,
    cache_region: RefCell<Option<String>>,
    cache_mode: Cell<Option<i64>>,
    result_set_mapping: RefCell<Option<ResultSetMapping>>,
}

impl std::fmt::Debug for StandInQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StandInQuery")
            .field("dql", &self.dql.borrow())
            .field("parameters", &self.parameters.borrow())
            .finish_non_exhaustive()
    }
}

impl StandInQuery {
    /// `$em->createQuery($dql)`.
    pub fn new(em: Arc<dyn QueryContext>, dql: &str) -> Self {
        Self {
            em,
            dql: RefCell::new(dql.to_owned()),
            parameters: RefCell::new(Vec::new()),
            hints: RefCell::new(Vec::new()),
            hydration_mode: Cell::new(HYDRATE_OBJECT),
            first_result: Cell::new(0),
            max_results: Cell::new(None),
            cacheable: Cell::new(false),
            lifetime: Cell::new(0),
            cache_region: RefCell::new(None),
            cache_mode: Cell::new(None),
            result_set_mapping: RefCell::new(None),
        }
    }

    /// The entity manager, as a handle.
    pub fn entity_manager(&self) -> Arc<dyn QueryContext> {
        Arc::clone(&self.em)
    }

    /// `setDQL($dql)`.
    pub fn set_dql(&self, dql: &str) -> &Self {
        dql.clone_into(&mut self.dql.borrow_mut());
        self
    }

    /// `setParameters($parameters)`.
    pub fn set_parameters(&self, parameters: Vec<Parameter>) -> &Self {
        *self.parameters.borrow_mut() = parameters;
        self
    }

    /// `getParameters()`.
    pub fn get_parameters(&self) -> Vec<Parameter> {
        self.parameters.borrow().clone()
    }

    /// `setParameter($key, $value, $type)`: an existing parameter of that
    /// name takes the value and type.
    pub fn set_parameter(
        &self,
        key: impl Into<ParameterName>,
        value: impl Into<Value>,
        type_: Option<WrapperParameterType>,
    ) -> &Self {
        let key = key.into();
        let name = Parameter::normalize_name(key.clone());
        let mut parameters = self.parameters.borrow_mut();
        match parameters.iter_mut().find(|p| p.get_name() == name) {
            Some(parameter) => parameter.set_value(value, type_),
            None => parameters.push(Parameter::new(key, value, type_)),
        }
        drop(parameters);
        self
    }

    /// `setFirstResult($firstResult)`: `null` is 0.
    pub fn set_first_result(&self, first_result: Option<i64>) -> &Self {
        self.first_result.set(first_result.unwrap_or(0));
        self
    }

    /// `setMaxResults($maxResults)`.
    pub fn set_max_results(&self, max_results: Option<i64>) -> &Self {
        self.max_results.set(max_results);
        self
    }

    /// `setHydrationMode($hydrationMode)`.
    pub fn set_hydration_mode(&self, mode: i64) -> &Self {
        self.hydration_mode.set(mode);
        self
    }

    /// `setHint($name, $value)` as a builder.
    pub fn hint(&self, name: &str, value: Hint) -> &Self {
        self.set_hint(name, value);
        self
    }

    /// `getHints()`, in the order they were set.
    pub fn get_hints(&self) -> Vec<(String, Hint)> {
        self.hints.borrow().clone()
    }

    /// `setCacheable($cacheable)`.
    pub fn set_cacheable(&self, cacheable: bool) -> &Self {
        self.cacheable.set(cacheable);
        self
    }

    /// `isCacheable()`.
    pub fn is_cacheable(&self) -> bool {
        self.cacheable.get()
    }

    /// `setLifetime($lifetime)`.
    pub fn set_lifetime(&self, lifetime: i64) -> &Self {
        self.lifetime.set(lifetime);
        self
    }

    /// `getLifetime()`.
    pub fn get_lifetime(&self) -> i64 {
        self.lifetime.get()
    }

    /// `setCacheRegion($cacheRegion)`.
    pub fn set_cache_region(&self, cache_region: &str) -> &Self {
        *self.cache_region.borrow_mut() = Some(cache_region.to_owned());
        self
    }

    /// `getCacheRegion()`.
    pub fn get_cache_region(&self) -> Option<String> {
        self.cache_region.borrow().clone()
    }

    /// `setCacheMode($cacheMode)`.
    pub fn set_cache_mode(&self, cache_mode: Option<i64>) -> &Self {
        self.cache_mode.set(cache_mode);
        self
    }

    /// `getCacheMode()`.
    pub fn get_cache_mode(&self) -> Option<i64> {
        self.cache_mode.get()
    }

    /// `setResultSetMapping($rsm)`.
    pub fn set_result_set_mapping(&self, rsm: ResultSetMapping) -> &Self {
        *self.result_set_mapping.borrow_mut() = Some(rsm);
        self
    }

    /// The result set mapping set with `setResultSetMapping()`.
    pub fn get_result_set_mapping(&self) -> Option<ResultSetMapping> {
        self.result_set_mapping.borrow().clone()
    }

    /// `clone $query`: `AbstractQuery::__clone()` drops the parameters
    /// and the hints (`Query::__clone()` also resets the parser state,
    /// which this stand-in does not keep).
    #[must_use]
    pub fn clone_query(&self) -> Self {
        let clone = Self::new(Arc::clone(&self.em), &self.dql.borrow());
        clone.hydration_mode.set(self.hydration_mode.get());
        clone.first_result.set(self.first_result.get());
        clone.max_results.set(self.max_results.get());
        clone.cacheable.set(self.cacheable.get());
        clone.lifetime.set(self.lifetime.get());
        clone
            .cache_region
            .borrow_mut()
            .clone_from(&self.cache_region.borrow());
        clone.cache_mode.set(self.cache_mode.get());
        clone
            .result_set_mapping
            .borrow_mut()
            .clone_from(&self.result_set_mapping.borrow());
        clone
    }

    /// `parse()`: a new parser's result (the stand-in does not cache it).
    pub fn parse(&self) -> Result<Rc<RefCell<ParserResult>>, Thrown> {
        Parser::new(self)?.parse()
    }

    /// `getSQL()`.
    pub fn get_sql(&self) -> Result<SqlStatements, Thrown> {
        let parser_result = self.parse()?;
        let executor = parser_result.borrow().prepare_sql_executor(self)?;
        Ok(executor.get_sql_statements().clone())
    }
}

impl DqlQuery for StandInQuery {
    fn get_entity_manager(&self) -> &dyn QueryContext {
        self.em.as_ref()
    }

    fn get_dql(&self) -> String {
        self.dql.borrow().clone()
    }

    fn get_hint(&self, name: &str) -> Hint {
        self.hints
            .borrow()
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Hint::Value(Value::Bool(false)), |(_, h)| h.clone())
    }

    fn set_hint(&self, name: &str, value: Hint) {
        let mut hints = self.hints.borrow_mut();
        match hints.iter_mut().find(|(n, _)| n == name) {
            Some((_, hint)) => *hint = value,
            None => hints.push((name.to_owned(), value)),
        }
    }

    fn has_hint(&self, name: &str) -> bool {
        self.hints
            .borrow()
            .iter()
            .any(|(n, h)| n == name && !matches!(h, Hint::Value(Value::Null)))
    }

    fn get_hydration_mode(&self) -> i64 {
        self.hydration_mode.get()
    }

    fn get_parameter(&self, key: &str) -> Option<Parameter> {
        let key = Parameter::normalize_name(key);
        self.parameters
            .borrow()
            .iter()
            .find(|p| p.get_name() == key)
            .cloned()
    }

    fn get_first_result(&self) -> i64 {
        self.first_result.get()
    }

    fn get_max_results(&self) -> Option<i64> {
        self.max_results.get()
    }
}
