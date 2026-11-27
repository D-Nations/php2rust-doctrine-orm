//! `AbstractQuery`: what DQL and native queries share, the parameters, the
//! hints, the hydration mode and the result set mapping, and the result
//! functions (`getResult()`, `getSingleResult()`, ...).
//!
//! The state is behind a lock, so a query's setters take `&self` (PHP's
//! fluent `$this`) and its futures are `Send`. A result is a [`Value`]:
//! the array of a hydration, the scalar of `HYDRATE_SINGLE_SCALAR`, or
//! the affected row count of a DQL UPDATE or DELETE. A cacheable query
//! of an em with the second-level cache goes through its query cache
//! (`executeUsingQueryCache()`). The result and hydration caches and
//! custom hydration modes are not ported; `toIterable()` hydrates row by row but reads the rows
//! whole.
//!
//! Upstream: doctrine/orm src/AbstractQuery.php src/NoResultException.php src/NonUniqueResultException.php src/UnexpectedResultException.php

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::WrapperParameterType;

use crate::collection_object::ArrayCollectionObject;
use crate::entity_manager::EntityManager;
use crate::internal::hydration::{self, HydrationHints};
use crate::persistent_collection::PersistentCollection;
use crate::query::context::{
    FORCE_DBAL_TYPE_CONVERSION, HINT_INTERNAL_ITERATION, HINT_READ_ONLY, HINT_REFRESH,
    HINT_REFRESH_ENTITY, Hint,
};
use crate::query::{Parameter, ParameterName, ResultSetMapping};
use crate::unit_of_work::{HINT_DEFEREAGERLOAD, Hints};
use crate::utility::{key_text, to_text};

pub use crate::query::context::{
    HYDRATE_ARRAY, HYDRATE_OBJECT, HYDRATE_SCALAR, HYDRATE_SCALAR_COLUMN, HYDRATE_SIMPLEOBJECT,
    HYDRATE_SINGLE_SCALAR,
};

/// `NoResultException::class`.
pub const NO_RESULT_EXCEPTION: &str = "Doctrine\\ORM\\NoResultException";
/// `NonUniqueResultException::class`.
pub const NON_UNIQUE_RESULT_EXCEPTION: &str = "Doctrine\\ORM\\NonUniqueResultException";
/// `UnexpectedResultException::class`.
pub const UNEXPECTED_RESULT_EXCEPTION: &str = "Doctrine\\ORM\\UnexpectedResultException";
/// `NonUniqueResultException::DEFAULT_MESSAGE`.
pub const NON_UNIQUE_RESULT_DEFAULT_MESSAGE: &str =
    "More than one result was found for query although one row or none was expected.";

/// `new NoResultException()`.
pub fn no_result() -> Thrown {
    Thrown::new(
        NO_RESULT_EXCEPTION,
        "No result was found for query although at least one row was expected.",
        0,
    )
}

/// `new NonUniqueResultException($message)`.
pub fn non_unique_result(message: Option<&str>) -> Thrown {
    Thrown::new(
        NON_UNIQUE_RESULT_EXCEPTION,
        message.unwrap_or(NON_UNIQUE_RESULT_DEFAULT_MESSAGE),
        0,
    )
}

/// Declares the query result exception classes in persistence's class
/// registry.
pub(crate) fn register_classes() {
    use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

    register_class(
        ClassInfo::class(UNEXPECTED_RESULT_EXCEPTION)
            .extends("RuntimeException")
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
    for class in [NO_RESULT_EXCEPTION, NON_UNIQUE_RESULT_EXCEPTION] {
        register_class(ClassInfo::class(class).extends(UNEXPECTED_RESULT_EXCEPTION));
    }
    register_class(
        ClassInfo::class(crate::exception::INVALID_HYDRATION_MODE)
            .extends("LogicException")
            .implements("Doctrine\\ORM\\Exception\\ManagerException"),
    );
    hydration::register_classes();
}

/// What `_doExecute()` gives: the rows of a SELECT, or the affected row
/// count of an UPDATE or DELETE.
#[derive(Debug, Clone)]
pub enum Executed {
    Rows(Vec<PhpArray>),
    Count(i64),
}

#[derive(Default)]
struct Inner {
    parameters: Vec<Parameter>,
    hints: Vec<(String, Hint)>,
    hydration_mode: i64,
    /// Bumped by `setHint()` and `setHydrationMode()`, which make a DQL
    /// query dirty (`Query::setHint()`, `Query::setHydrationMode()`).
    changes: u64,
    result_set_mapping: Option<ResultSetMapping>,
    // The second-level cache options.
    cacheable: bool,
    lifetime: i64,
    cache_region: Option<String>,
    cache_mode: Option<i64>,
}

/// The state of an `AbstractQuery`.
pub struct QueryState {
    em: Arc<EntityManager>,
    inner: Mutex<Inner>,
}

impl std::fmt::Debug for QueryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryState").finish_non_exhaustive()
    }
}

/// `$hint !== null` for `isset($hints[$name])`.
fn isset(hint: Option<&Hint>) -> bool {
    match hint {
        None | Some(Hint::Value(Value::Null)) => false,
        Some(_) => true,
    }
}

fn default_hints(em: &EntityManager) -> Vec<(String, Hint)> {
    em.get_configuration()
        .get_default_query_hints()
        .iter()
        .map(|(k, v)| (key_text(k), Hint::Value(v.clone())))
        .collect()
}

impl QueryState {
    /// `new AbstractQuery($em)`.
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            em: em.clone(),
            inner: Mutex::new(Inner {
                hints: default_hints(em),
                hydration_mode: crate::query::context::HYDRATE_OBJECT,
                ..Inner::default()
            }),
        }
    }

    fn st(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `__clone()`: the clone has no parameters and the default hints.
    #[must_use]
    pub fn clone_state(&self) -> Self {
        let st = self.st();
        Self {
            em: self.em.clone(),
            inner: Mutex::new(Inner {
                parameters: Vec::new(),
                hints: default_hints(&self.em),
                hydration_mode: st.hydration_mode,
                changes: 0,
                result_set_mapping: st.result_set_mapping.clone(),
                cacheable: st.cacheable,
                lifetime: st.lifetime,
                cache_region: st.cache_region.clone(),
                cache_mode: st.cache_mode,
            }),
        }
    }

    /// Retrieves the associated `EntityManager` of this query instance.
    pub fn get_entity_manager(&self) -> &Arc<EntityManager> {
        &self.em
    }

    /// Frees the resources used by the query object (the parameters).
    pub fn free(&self) {
        self.st().parameters.clear();
    }

    /// Get all defined parameters.
    pub fn get_parameters(&self) -> Vec<Parameter> {
        self.st().parameters.clone()
    }

    /// Gets a query parameter.
    pub fn get_parameter(&self, key: impl Into<ParameterName>) -> Option<Parameter> {
        let key = Parameter::normalize_name(key);
        self.st()
            .parameters
            .iter()
            .find(|p| p.get_name() == key)
            .cloned()
    }

    /// Sets a collection of query parameters.
    pub fn set_parameters(&self, parameters: Vec<Parameter>) {
        self.st().parameters = parameters;
    }

    /// Sets the query parameters of an array (`new Parameter($key, $value)`
    /// for each entry).
    pub fn set_parameters_array(&self, parameters: &PhpArray) {
        let parameters = parameters
            .iter()
            .map(|(k, v)| {
                let name = match k {
                    Key::Int(i) => ParameterName::Int(*i),
                    Key::Str(_) => ParameterName::String(key_text(k)),
                };
                Parameter::new(name, v.clone(), None)
            })
            .collect();
        self.set_parameters(parameters);
    }

    /// Sets a query parameter.
    pub fn set_parameter(
        &self,
        key: impl Into<ParameterName>,
        value: impl Into<Value>,
        type_: Option<WrapperParameterType>,
    ) {
        let key = key.into();
        let name = Parameter::normalize_name(key.clone());
        let mut st = self.st();
        if let Some(existing) = st.parameters.iter_mut().find(|p| p.get_name() == name) {
            existing.set_value(value, type_);
            return;
        }
        st.parameters.push(Parameter::new(key, value, type_));
    }

    /// Processes an individual parameter value.
    pub fn process_parameter_value(&self, value: &Value) -> Result<Value, Thrown> {
        match value {
            Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_) => {
                Ok(value.clone())
            }
            Value::Array(array) => Ok(Value::Array(self.process_array_parameter_value(array)?)),
            Value::Object(object) => {
                if let Some(collection) = ArrayCollectionObject::from_value(value) {
                    let elements = collection.lock().to_array();
                    return Ok(Value::Array(self.process_array_parameter_value(&elements)?));
                }
                if let Some(collection) = PersistentCollection::from_value(value) {
                    let elements = collection.unwrap();
                    return Ok(Value::Array(self.process_array_parameter_value(&elements)?));
                }
                if let Some(backing) = object
                    .downcast_ref::<php_value::object::EnumCase>()
                    .and_then(php_value::object::EnumCase::value)
                {
                    return Ok(backing.clone());
                }
                if let Some(metadata) =
                    crate::collection_object::downcast::<crate::mapping::ClassMetadata>(object)
                {
                    return Ok(Value::from(metadata.read().name.as_str()));
                }
                // Silence any mapping exceptions. These can occur if the
                // object in question is not a mapped entity, in which case
                // we just don't do any preparation on the value.
                let class = object.class().to_owned();
                if self.em.is_transient(&class).unwrap_or(true) {
                    return Ok(value.clone());
                }
                let id = self
                    .em
                    .get_unit_of_work()
                    .get_single_identifier_value(object)?;
                if id.is_null() {
                    return Err(crate::exception::invalid_identifier_binding_entity(&class));
                }
                Ok(id)
            }
        }
    }

    /// Process a parameter value which was previously identified as an
    /// array.
    fn process_array_parameter_value(&self, value: &PhpArray) -> Result<PhpArray, Thrown> {
        let mut processed = PhpArray::new();
        for (key, param_value) in value.iter() {
            let param_value = self.process_parameter_value(param_value)?;
            let param_value = match param_value {
                // reset($paramValue)
                Value::Array(array) => array.values().next().cloned().unwrap_or(Value::Bool(false)),
                other => other,
            };
            processed.insert(key.clone(), param_value);
        }
        Ok(processed)
    }

    /// Sets the `ResultSetMapping` that should be used for hydration.
    pub fn set_result_set_mapping(&self, mut rsm: ResultSetMapping) -> Result<(), Thrown> {
        // translateNamespaces()
        for (_, class_name) in rsm.alias_map.iter_mut() {
            let class = self.em.get_class_metadata(class_name)?;
            class_name.clone_from(&class.read().name);
        }
        for (_, class_name) in rsm.declaring_classes.iter_mut() {
            let class = self.em.get_class_metadata(class_name)?;
            class_name.clone_from(&class.read().name);
        }
        self.st().result_set_mapping = Some(rsm);
        Ok(())
    }

    /// Gets the `ResultSetMapping` used for hydration.
    pub fn get_result_set_mapping(&self) -> Option<ResultSetMapping> {
        self.st().result_set_mapping.clone()
    }

    /// The mapping, set from the parser's when none is set yet.
    pub(crate) fn result_set_mapping_or(&self, rsm: impl FnOnce() -> ResultSetMapping) {
        let mut st = self.st();
        if st.result_set_mapping.is_none() {
            st.result_set_mapping = Some(rsm());
        }
    }

    /// Change the default fetch mode of an association for this query.
    pub fn set_fetch_mode(&self, class: &str, assoc_name: &str, fetch_mode: i64) {
        let mut st = self.st();
        let position = st.hints.iter().position(|(n, _)| n == "fetchMode");
        let mut modes = match position.map(|p| &st.hints[p].1) {
            Some(Hint::Value(Value::Array(modes))) => modes.clone(),
            _ => PhpArray::new(),
        };
        let class_key = Key::from(class);
        let mut class_modes = match modes.get(&class_key) {
            Some(Value::Array(class_modes)) => class_modes.clone(),
            _ => PhpArray::new(),
        };
        class_modes.insert(assoc_name, Value::Int(fetch_mode));
        modes.insert(class_key, Value::Array(class_modes));
        let hint = Hint::Value(Value::Array(modes));
        match position {
            Some(p) => st.hints[p].1 = hint,
            None => st.hints.push(("fetchMode".to_owned(), hint)),
        }
    }

    /// Defines the processing mode to be used during hydration /
    /// result set transformation.
    pub fn set_hydration_mode(&self, hydration_mode: i64) {
        let mut st = self.st();
        st.hydration_mode = hydration_mode;
        st.changes += 1;
    }

    /// How many times a hint or the hydration mode was set: a DQL query
    /// is dirty when this changed since it was parsed.
    pub fn changes(&self) -> u64 {
        self.st().changes
    }

    /// Gets the hydration mode currently used by the query.
    pub fn get_hydration_mode(&self) -> i64 {
        self.st().hydration_mode
    }

    /// Sets a query hint. If the hint name is not recognized, it is
    /// silently ignored.
    pub fn set_hint(&self, name: &str, value: Hint) {
        let mut st = self.st();
        st.changes += 1;
        match st.hints.iter_mut().find(|(n, _)| n == name) {
            Some((_, hint)) => *hint = value,
            None => st.hints.push((name.to_owned(), value)),
        }
    }

    /// Gets the value of a query hint. If the hint name is not
    /// recognized, FALSE is returned.
    pub fn get_hint(&self, name: &str) -> Hint {
        self.st()
            .hints
            .iter()
            .find(|(n, _)| n == name)
            .map_or(Hint::Value(Value::Bool(false)), |(_, h)| h.clone())
    }

    pub fn has_hint(&self, name: &str) -> bool {
        isset(
            self.st()
                .hints
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, h)| h),
        )
    }

    /// Return the key value map of query hints that are currently set.
    pub fn get_hints(&self) -> Vec<(String, Hint)> {
        self.st().hints.clone()
    }

    /// Enable/disable second level query (result) caching for this query.
    pub fn set_cacheable(&self, cacheable: bool) {
        self.st().cacheable = cacheable;
    }

    pub fn is_cacheable(&self) -> bool {
        self.st().cacheable
    }

    pub fn set_cache_region(&self, cache_region: &str) {
        self.st().cache_region = Some(cache_region.to_owned());
    }

    /// Obtain the name of the second level query cache region in which
    /// query results will be stored.
    pub fn get_cache_region(&self) -> Option<String> {
        self.st().cache_region.clone()
    }

    /// Sets the life-time for this query into second level cache.
    pub fn set_lifetime(&self, lifetime: i64) {
        self.st().lifetime = lifetime;
    }

    pub fn get_lifetime(&self) -> i64 {
        self.st().lifetime
    }

    pub fn get_cache_mode(&self) -> Option<i64> {
        self.st().cache_mode
    }

    pub fn set_cache_mode(&self, cache_mode: Option<i64>) {
        self.st().cache_mode = cache_mode;
    }

    /// `$this->hasCache`: whether the em has the second-level cache.
    pub fn has_cache(&self) -> bool {
        self.em.get_configuration().is_second_level_cache_enabled()
    }

    /// `$this->cacheLogger`.
    pub(crate) fn cache_logger(&self) -> Option<Arc<dyn crate::cache::logging::CacheLogger>> {
        if !self.has_cache() {
            return None;
        }
        self.em
            .get_configuration()
            .get_second_level_cache_configuration()
            .and_then(|config| config.get_cache_logger())
    }

    /// `$this->hints` as a PHP array: a lock mode is its integer, the
    /// walkers are their class names.
    pub fn hints_array(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for (name, hint) in &self.st().hints {
            let value = match hint {
                Hint::Value(value) => value.clone(),
                Hint::LockMode(mode) => Value::Int(match mode {
                    php2rust_doctrine_dbal::LockMode::None => 0,
                    php2rust_doctrine_dbal::LockMode::Optimistic => 1,
                    php2rust_doctrine_dbal::LockMode::PessimisticRead => 2,
                    php2rust_doctrine_dbal::LockMode::PessimisticWrite => 4,
                }),
                Hint::TreeWalkers(walkers) => Value::Array(PhpArray::list(
                    walkers.iter().map(|w| Value::string(w.name)),
                )),
                Hint::OutputWalker(walker) => Value::string(walker.name),
            };
            array.insert(Key::from(name.as_str()), value);
        }
        array
    }

    /// `AbstractQuery::getHash()` of a query's SQL: `sha1($query . '-' .
    /// serialize($params) . '-' . serialize($hints))`, the hints sorted
    /// by name.
    pub fn get_hash(&self, query: &str) -> Result<String, Thrown> {
        let mut params = Vec::new();
        for parameter in self.get_parameters() {
            let value = parameter.get_value();
            // Small optimization
            // Does not invoke processParameterValue for scalar value
            params.push(match value {
                Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_) => {
                    value.clone()
                }
                _ => self.process_parameter_value(value)?,
            });
        }
        let mut hints = self.hints_array();
        hints.ksort();
        let mut data = format!("{query}-").into_bytes();
        data.extend_from_slice(
            php_value::serialize::serialize(&Value::Array(PhpArray::list(params))).as_bytes(),
        );
        data.push(b'-');
        data.extend_from_slice(php_value::serialize::serialize(&Value::Array(hints)).as_bytes());
        Ok(crate::cache::sha1_hex(&data))
    }

    /// `getTimestampKey()`: the key of the first alias's root entity.
    pub(crate) fn get_timestamp_key(
        &self,
        rsm: &ResultSetMapping,
    ) -> Result<Option<crate::cache::TimestampCacheKey>, Thrown> {
        let Some((_, entity_name)) = rsm.alias_map.iter().next() else {
            return Ok(None);
        };
        if entity_name.is_empty() {
            return Ok(None);
        }
        let metadata = self.em.get_class_metadata(entity_name)?;
        let root = metadata.read().root_entity_name.clone();
        Ok(Some(crate::cache::TimestampCacheKey::new(root)))
    }

    /// `executeUsingQueryCache()` around the query's execution
    /// (`executeIgnoreQueryCache()`), with its mapping and hash.
    pub(crate) async fn execute_using_query_cache<F>(
        &self,
        rsm: ResultSetMapping,
        hash: String,
        execute: F,
    ) -> Result<Value, Thrown>
    where
        F: std::future::Future<Output = Result<Value, Thrown>>,
    {
        let cache = self.em.get_cache().ok_or_else(|| {
            Thrown::new(
                "LogicException",
                "The second level cache is not enabled.",
                0,
            )
        })?;
        let (cache_region, lifetime, cache_mode) = {
            let st = self.st();
            (st.cache_region.clone(), st.lifetime, st.cache_mode)
        };
        let query_cache = cache.get_query_cache(cache_region.as_deref())?;
        let query_key = crate::cache::QueryCacheKey::with(
            hash,
            lifetime,
            cache_mode
                .filter(|mode| *mode != 0)
                .unwrap_or(crate::cache::MODE_NORMAL),
            self.get_timestamp_key(&rsm)?,
        );
        let cache_logger = self.cache_logger();

        let hints = self.hints_array();
        let result = query_cache.get(&query_key, &rsm, &hints).await?;

        if let Some(result) = result {
            if let Some(logger) = &cache_logger {
                logger.query_cache_hit(&query_cache.get_region().get_name(), &query_key);
            }
            return Ok(Value::Array(result));
        }

        let result = execute.await?;
        let hints = self.hints_array();
        let array = match &result {
            Value::Array(array) => array.clone(),
            _ => PhpArray::new(),
        };
        let cached = query_cache.put(&query_key, &rsm, &array, &hints).await?;

        if let Some(logger) = &cache_logger {
            logger.query_cache_miss(&query_cache.get_region().get_name(), &query_key);
            if cached {
                logger.query_cache_put(&query_cache.get_region().get_name(), &query_key);
            }
        }

        Ok(result)
    }

    /// The query hints as the unit of work and the hydrators read them.
    pub(crate) fn hydration_hints(&self) -> (Hints, HydrationHints) {
        let st = self.st();
        let get = |name: &str| st.hints.iter().find(|(n, _)| n == name).map(|(_, h)| h);
        let mut fetch_mode = Vec::new();
        if let Some(Hint::Value(Value::Array(modes))) = get("fetchMode") {
            for (class, assocs) in modes.iter() {
                if let Value::Array(assocs) = assocs {
                    for (assoc, mode) in assocs.iter() {
                        let mode = match mode {
                            Value::Int(i) => Some(*i),
                            Value::Null => None,
                            other => to_text(other).parse().ok(),
                        };
                        fetch_mode.push((key_text(class), key_text(assoc), mode));
                    }
                }
            }
        }
        let refresh_entity = match get(HINT_REFRESH_ENTITY) {
            Some(Hint::Value(Value::Object(entity))) => Some(entity.clone()),
            _ => None,
        };
        let defer = get(HINT_DEFEREAGERLOAD);
        let hints = Hints {
            refresh: isset(get(HINT_REFRESH)),
            refresh_entity,
            read_only: get(HINT_READ_ONLY).is_some_and(Hint::is_true),
            defer_eager_load: isset(defer),
            fetch_mode,
            force_dbal_type_conversion: isset(get(FORCE_DBAL_TYPE_CONVERSION)),
            internal_iteration: get(HINT_INTERNAL_ITERATION).is_some_and(Hint::is_truthy),
            ..Hints::default()
        };
        let extra = HydrationHints {
            defer_eager_load: isset(defer).then(|| defer.is_some_and(Hint::is_true)),
            collection: None,
        };
        (hints, extra)
    }

    /// `$this->em->newHydrator($this->hydrationMode)->hydrateAll($stmt,
    /// $rsm, $this->hints)`.
    pub(crate) async fn hydrate(&self, rows: Vec<PhpArray>) -> Result<Value, Thrown> {
        let rsm = self
            .get_result_set_mapping()
            .ok_or_else(|| Thrown::new("LogicException", "Uninitialized result set mapping.", 0))?;
        let (mut hints, extra) = self.hydration_hints();
        let mode = self.get_hydration_mode();
        hydration::hydrate(&self.em, mode, rows, &rsm, &mut hints, extra).await
    }

    /// `$this->em->newHydrator($this->hydrationMode)->toIterable($stmt,
    /// $rsm, $this->hints)`.
    pub(crate) async fn hydrate_iterable(&self, rows: Vec<PhpArray>) -> Result<Vec<Value>, Thrown> {
        let rsm = self
            .get_result_set_mapping()
            .ok_or_else(|| Thrown::new("LogicException", "Uninitialized result set mapping.", 0))?;
        let (mut hints, extra) = self.hydration_hints();
        let mode = self.get_hydration_mode();
        hydration::to_iterable(&self.em, mode, rows, &rsm, &mut hints, extra).await
    }

    /// `hydrate_iterable()` with the generator's keys.
    pub(crate) async fn hydrate_iterable_with_keys(
        &self,
        rows: Vec<PhpArray>,
    ) -> Result<Vec<(Key, Value)>, Thrown> {
        let rsm = self
            .get_result_set_mapping()
            .ok_or_else(|| Thrown::new("LogicException", "Uninitialized result set mapping.", 0))?;
        let (mut hints, extra) = self.hydration_hints();
        let mode = self.get_hydration_mode();
        hydration::to_iterable_with_keys(&self.em, mode, rows, &rsm, &mut hints, extra).await
    }

    /// `getSingleResult()` of an executed result.
    pub(crate) fn single_result(&self, result: Value) -> Result<Value, Thrown> {
        if self.get_hydration_mode() != HYDRATE_SINGLE_SCALAR && !result.is_truthy() {
            return Err(no_result());
        }
        let Value::Array(result) = result else {
            return Ok(result);
        };
        if result.len() > 1 {
            return Err(non_unique_result(None));
        }
        Ok(result.values().next().cloned().unwrap_or(Value::Null))
    }

    /// `getOneOrNullResult()` of an executed result (`NoResultException`
    /// gives `null`).
    pub(crate) fn one_or_null_result(
        &self,
        result: Result<Value, Thrown>,
    ) -> Result<Value, Thrown> {
        let result = match result {
            Err(e) if e.class == NO_RESULT_EXCEPTION => return Ok(Value::Null),
            other => other?,
        };
        if self.get_hydration_mode() != HYDRATE_SINGLE_SCALAR && !result.is_truthy() {
            return Ok(Value::Null);
        }
        let Value::Array(result) = result else {
            return Ok(result);
        };
        if result.len() > 1 {
            return Err(non_unique_result(None));
        }
        Ok(result.values().next().cloned().unwrap_or(Value::Null))
    }
}

/// The `AbstractQuery` methods of a query type with a `state()` (its
/// [`QueryState`]) and an `async fn do_execute()` (`_doExecute()`).
macro_rules! abstract_query_methods {
    () => {
        /// Retrieves the associated `EntityManager` of this query instance.
        pub fn get_entity_manager(&self) -> &std::sync::Arc<$crate::entity_manager::EntityManager> {
            self.state().get_entity_manager()
        }

        /// Get all defined parameters.
        pub fn get_parameters(&self) -> Vec<$crate::query::Parameter> {
            self.state().get_parameters()
        }

        /// Gets a query parameter.
        pub fn get_parameter(
            &self,
            key: impl Into<$crate::query::ParameterName>,
        ) -> Option<$crate::query::Parameter> {
            self.state().get_parameter(key)
        }

        /// Sets a collection of query parameters.
        pub fn set_parameters(&self, parameters: Vec<$crate::query::Parameter>) -> &Self {
            self.state().set_parameters(parameters);
            self
        }

        /// Sets the query parameters of an array.
        pub fn set_parameters_array(&self, parameters: &php_value::PhpArray) -> &Self {
            self.state().set_parameters_array(parameters);
            self
        }

        /// Sets a query parameter.
        pub fn set_parameter(
            &self,
            key: impl Into<$crate::query::ParameterName>,
            value: impl Into<php_value::Value>,
            type_: Option<php2rust_doctrine_dbal::WrapperParameterType>,
        ) -> &Self {
            self.state().set_parameter(key, value, type_);
            self
        }

        /// Processes an individual parameter value.
        pub fn process_parameter_value(
            &self,
            value: &php_value::Value,
        ) -> Result<php_value::Value, php_value::Thrown> {
            self.state().process_parameter_value(value)
        }

        /// Sets the `ResultSetMapping` that should be used for hydration.
        pub fn set_result_set_mapping(
            &self,
            rsm: $crate::query::ResultSetMapping,
        ) -> Result<&Self, php_value::Thrown> {
            self.state().set_result_set_mapping(rsm)?;
            Ok(self)
        }

        /// Change the default fetch mode of an association for this query.
        pub fn set_fetch_mode(&self, class: &str, assoc_name: &str, fetch_mode: i64) -> &Self {
            self.state().set_fetch_mode(class, assoc_name, fetch_mode);
            self
        }

        /// Defines the processing mode to be used during hydration.
        pub fn set_hydration_mode(&self, hydration_mode: i64) -> &Self {
            self.state().set_hydration_mode(hydration_mode);
            self
        }

        /// Sets a query hint.
        pub fn set_hint(&self, name: &str, value: $crate::query::context::Hint) -> &Self {
            self.state().set_hint(name, value);
            self
        }

        /// Gets the hydration mode currently used by the query.
        pub fn get_hydration_mode(&self) -> i64 {
            self.state().get_hydration_mode()
        }

        /// Gets the value of a query hint (`false` when not set).
        pub fn get_hint(&self, name: &str) -> $crate::query::context::Hint {
            self.state().get_hint(name)
        }

        pub fn has_hint(&self, name: &str) -> bool {
            self.state().has_hint(name)
        }

        /// The query hints that are currently set.
        pub fn get_hints(&self) -> Vec<(String, $crate::query::context::Hint)> {
            self.state().get_hints()
        }

        /// Enable/disable second level query (result) caching for this query.
        pub fn set_cacheable(&self, cacheable: bool) -> &Self {
            self.state().set_cacheable(cacheable);
            self
        }

        pub fn is_cacheable(&self) -> bool {
            self.state().is_cacheable()
        }

        pub fn set_cache_region(&self, cache_region: &str) -> &Self {
            self.state().set_cache_region(cache_region);
            self
        }

        pub fn get_cache_region(&self) -> Option<String> {
            self.state().get_cache_region()
        }

        pub fn set_lifetime(&self, lifetime: i64) -> &Self {
            self.state().set_lifetime(lifetime);
            self
        }

        pub fn get_lifetime(&self) -> i64 {
            self.state().get_lifetime()
        }

        pub fn get_cache_mode(&self) -> Option<i64> {
            self.state().get_cache_mode()
        }

        pub fn set_cache_mode(&self, cache_mode: Option<i64>) -> &Self {
            self.state().set_cache_mode(cache_mode);
            self
        }

        /// Executes the query.
        pub async fn execute(
            &self,
            parameters: Option<&php_value::PhpArray>,
            hydration_mode: Option<i64>,
        ) -> Result<php_value::Value, php_value::Thrown> {
            if self.state().is_cacheable() && self.state().has_cache() {
                let rsm = self.get_result_set_mapping()?;
                let hash = self.get_hash()?;
                return self
                    .state()
                    .execute_using_query_cache(
                        rsm,
                        hash,
                        self.execute_ignore_query_cache(parameters, hydration_mode),
                    )
                    .await;
            }
            self.execute_ignore_query_cache(parameters, hydration_mode)
                .await
        }

        /// Execute query ignoring second level cache.
        async fn execute_ignore_query_cache(
            &self,
            parameters: Option<&php_value::PhpArray>,
            hydration_mode: Option<i64>,
        ) -> Result<php_value::Value, php_value::Thrown> {
            if let Some(mode) = hydration_mode {
                self.set_hydration_mode(mode);
            }
            if let Some(parameters) = parameters.filter(|p| !p.is_empty()) {
                self.state().set_parameters_array(parameters);
            }
            match self.do_execute().await? {
                $crate::abstract_query::Executed::Count(count) => Ok(php_value::Value::Int(count)),
                $crate::abstract_query::Executed::Rows(rows) => self.state().hydrate(rows).await,
            }
        }

        /// Gets the list of results for the query.
        pub async fn get_result(
            &self,
            hydration_mode: i64,
        ) -> Result<php_value::Value, php_value::Thrown> {
            self.execute(None, Some(hydration_mode)).await
        }

        /// `getResult()`: the result in `HYDRATE_OBJECT` mode.
        pub async fn get_object_result(&self) -> Result<php_value::Value, php_value::Thrown> {
            self.execute(None, Some($crate::query::context::HYDRATE_OBJECT))
                .await
        }

        /// Gets the array of results for the query.
        pub async fn get_array_result(&self) -> Result<php_value::Value, php_value::Thrown> {
            self.execute(None, Some($crate::query::context::HYDRATE_ARRAY))
                .await
        }

        /// Gets one-dimensional array of results for the query.
        pub async fn get_single_column_result(
            &self,
        ) -> Result<php_value::Value, php_value::Thrown> {
            self.execute(None, Some($crate::query::context::HYDRATE_SCALAR_COLUMN))
                .await
        }

        /// Gets the scalar results for the query.
        pub async fn get_scalar_result(&self) -> Result<php_value::Value, php_value::Thrown> {
            self.execute(None, Some($crate::query::context::HYDRATE_SCALAR))
                .await
        }

        /// Get exactly one result or null.
        pub async fn get_one_or_null_result(
            &self,
            hydration_mode: Option<i64>,
        ) -> Result<php_value::Value, php_value::Thrown> {
            let result = self.execute(None, hydration_mode).await;
            self.state().one_or_null_result(result)
        }

        /// Gets the single result of the query.
        pub async fn get_single_result(
            &self,
            hydration_mode: Option<i64>,
        ) -> Result<php_value::Value, php_value::Thrown> {
            let result = self.execute(None, hydration_mode).await?;
            self.state().single_result(result)
        }

        /// Gets the single scalar result of the query.
        pub async fn get_single_scalar_result(
            &self,
        ) -> Result<php_value::Value, php_value::Thrown> {
            self.get_single_result(Some($crate::query::context::HYDRATE_SINGLE_SCALAR))
                .await
        }
    };
}

pub(crate) use abstract_query_methods;
