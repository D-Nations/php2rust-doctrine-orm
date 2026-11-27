//! `Doctrine\ORM\Query`: a DQL query.
//!
//! The query keeps the parser result of a clean query and caches it in
//! the query cache, as PHP does; the walkers are not `Send`, so they
//! never live across an `await`, and the parser result is shared once
//! parsed. The query keeps PHP's result set mapping once a parse or
//! `setResultSetMapping()` set it. The query cache id hashes the hints'
//! debug form where PHP hashes `serialize($this->hints)`: the same hints
//! give the same id.
//!
//! Upstream: doctrine/orm src/Query.php

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::{LockMode, ParameterTypes, WrapperParameterType};

use crate::abstract_query::{Executed, QueryState, abstract_query_methods};
use crate::cache::pool::{CacheItemPool, CachedParserResult, PoolValue};
use crate::entity_manager::EntityManager;
use crate::query::context::{
    DqlQuery, HINT_INTERNAL_ITERATION, HINT_LOCK_MODE, Hint, OutputWalkerClass, QueryComponents,
    QueryContext,
};
use crate::query::exec::{SqlExecutor, SqlStatements};
use crate::query::{Parameter, ParameterTypeInferer, Parser, ParserResult, query_exception};

pub use crate::query::context::{
    HINT_CACHE_ENABLED, HINT_CACHE_EVICT, HINT_CUSTOM_OUTPUT_WALKER, HINT_CUSTOM_TREE_WALKERS,
    HINT_FORCE_PARTIAL_LOAD, HINT_INCLUDE_META_COLUMNS, HINT_READ_ONLY, HINT_REFRESH,
    HINT_REFRESH_ENTITY,
};

/// `md5($data)`.
fn md5_hex(data: &[u8]) -> String {
    use md5::Digest as _;
    use std::fmt::Write as _;
    md5::Md5::digest(data)
        .iter()
        .fold(String::with_capacity(32), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}

/// `Query::STATE_CLEAN`.
pub const STATE_CLEAN: i64 = 1;
/// `Query::STATE_DIRTY`.
pub const STATE_DIRTY: i64 = 2;

struct DqlState {
    dql: Option<String>,
    first_result: i64,
    max_results: Option<i64>,
    state: i64,
    /// `$parserResult`.
    parser_result: Option<Arc<ParserResult>>,
    /// `$parsedTypes`: the parameter types of the last parse.
    parsed_types: Vec<(String, WrapperParameterType)>,
    /// The hint and hydration mode changes at the last parse
    /// (`QueryState::changes()`).
    parsed_changes: u64,
    /// The cache driver used for caching queries.
    query_cache: Option<Arc<dyn CacheItemPool>>,
    /// Whether or not expire the query cache.
    expire_query_cache: bool,
    /// The query cache lifetime.
    query_cache_ttl: Option<i64>,
    /// Whether to use a query cache, if available. Defaults to TRUE.
    use_query_cache: bool,
}

impl DqlState {
    fn new(dql: Option<String>, first_result: i64, max_results: Option<i64>, state: i64) -> Self {
        Self {
            dql,
            first_result,
            max_results,
            state,
            parser_result: None,
            parsed_types: Vec::new(),
            parsed_changes: 0,
            query_cache: None,
            expire_query_cache: false,
            query_cache_ttl: None,
            use_query_cache: true,
        }
    }
}

/// A `Query` object represents a DQL query.
pub struct Query {
    state: QueryState,
    dql: Mutex<DqlState>,
}

impl std::fmt::Debug for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Query")
            .field("dql", &self.get_dql())
            .finish_non_exhaustive()
    }
}

impl Query {
    /// `new Query($em)`.
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            state: QueryState::new(em),
            dql: Mutex::new(DqlState::new(None, 0, None, STATE_CLEAN)),
        }
    }

    fn dst(&self) -> MutexGuard<'_, DqlState> {
        self.dql.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The `AbstractQuery` state.
    pub fn state(&self) -> &QueryState {
        &self.state
    }

    /// `clone $query`: the same DQL, first and max results, without the
    /// parameters, with the default hints.
    #[must_use]
    pub fn clone_query(&self) -> Self {
        let dst = self.dst();
        let mut clone = DqlState::new(
            dst.dql.clone(),
            dst.first_result,
            dst.max_results,
            STATE_DIRTY,
        );
        clone.parser_result.clone_from(&dst.parser_result);
        clone.query_cache.clone_from(&dst.query_cache);
        clone.expire_query_cache = dst.expire_query_cache;
        clone.query_cache_ttl = dst.query_cache_ttl;
        clone.use_query_cache = dst.use_query_cache;
        Self {
            state: self.state.clone_state(),
            dql: Mutex::new(clone),
        }
    }

    abstract_query_methods!();

    /// Gets the SQL query/queries that correspond to this DQL query.
    pub fn get_sql(&self) -> Result<SqlStatements, Thrown> {
        Ok(self.get_sql_executor()?.get_sql_statements().clone())
    }

    /// `getSQL()` of a query with one statement (the statements joined
    /// with `"; "` otherwise).
    pub fn get_single_sql(&self) -> Result<String, Thrown> {
        Ok(match self.get_sql()? {
            SqlStatements::One(sql) => sql,
            SqlStatements::Many(sql) => sql.join("; "),
        })
    }

    fn get_sql_executor(&self) -> Result<SqlExecutor, Thrown> {
        let parser_result = self.parse()?;
        self.state
            .result_set_mapping_or(|| parser_result.get_result_set_mapping().clone());
        parser_result.prepare_sql_executor(self)
    }

    /// `getResultSetMapping()`: the mapping, parsing the query when none
    /// is set.
    fn get_result_set_mapping(&self) -> Result<crate::query::ResultSetMapping, Thrown> {
        if let Some(rsm) = self.state.get_result_set_mapping() {
            return Ok(rsm);
        }
        let rsm = self.parse()?.get_result_set_mapping().clone();
        self.state.result_set_mapping_or(|| rsm.clone());
        Ok(self.state.get_result_set_mapping().unwrap_or(rsm))
    }

    /// `getHash()`: `sha1(parent::getHash() . '-' . $this->firstResult .
    /// '-' . $this->maxResults)`. Public, as the tests call it by
    /// reflection.
    pub fn get_hash(&self) -> Result<String, Thrown> {
        let sql = match self.get_sql()? {
            SqlStatements::One(sql) => sql,
            // `$query . '-'` of an array.
            SqlStatements::Many(_) => "Array".to_owned(),
        };
        let parent = self.state.get_hash(&sql)?;
        let (first_result, max_results) = {
            let dst = self.dst();
            (dst.first_result, dst.max_results)
        };
        let max_results = max_results.map(|m| m.to_string()).unwrap_or_default();
        Ok(crate::cache::sha1_hex(
            format!("{parent}-{first_result}-{max_results}").as_bytes(),
        ))
    }

    /// Parses the DQL query, if necessary, and stores the parser result.
    fn parse(&self) -> Result<Arc<ParserResult>, Thrown> {
        let types: Vec<(String, WrapperParameterType)> = self
            .state
            .get_parameters()
            .iter()
            .map(|parameter| {
                (
                    parameter.get_name().to_owned(),
                    parameter.get_type().clone(),
                )
            })
            .collect();
        let changes = self.state.changes();

        // Return previous parser result if the query and the filter collection are both clean
        {
            let dst = self.dst();
            if dst.state == STATE_CLEAN
                && dst.parsed_changes == changes
                && dst.parsed_types == types
                && self.get_entity_manager().is_filters_state_clean()
                && let Some(parser_result) = &dst.parser_result
            {
                return Ok(parser_result.clone());
            }
        }

        let (query_cache, use_query_cache, expire_query_cache, query_cache_ttl) = {
            let mut dst = self.dst();
            dst.state = STATE_CLEAN;
            dst.parsed_types = types;
            dst.parsed_changes = changes;
            (
                dst.query_cache.clone(),
                dst.use_query_cache,
                dst.expire_query_cache,
                dst.query_cache_ttl,
            )
        };

        let query_cache = query_cache.or_else(|| {
            self.get_entity_manager()
                .get_configuration()
                .get_query_cache()
        });
        // Check query cache.
        let Some(query_cache) = query_cache.filter(|_| use_query_cache) else {
            return self.parse_dql();
        };

        let cache_item = query_cache.get_item(&self.get_query_cache_id())?;

        if !expire_query_cache
            && cache_item.is_hit()
            && let Some(cached) = cache_item
                .get()
                .and_then(PoolValue::downcast_ref::<CachedParserResult>)
        {
            // Cache hit.
            let parser_result = cached.0.clone();
            self.dst().parser_result = Some(parser_result.clone());
            return Ok(parser_result);
        }

        // Cache miss.
        let parser_result = self.parse_dql()?;

        let mut cache_item =
            cache_item.set(PoolValue::object(CachedParserResult(parser_result.clone())));
        if let Some(ttl) = query_cache_ttl {
            cache_item = cache_item.expires_after(ttl);
        }
        query_cache.save(cache_item);

        Ok(parser_result)
    }

    /// `$this->parserResult = (new Parser($this))->parse()`.
    fn parse_dql(&self) -> Result<Arc<ParserResult>, Thrown> {
        let parser_result = Parser::new(self)?.parse()?;
        let parser_result = Arc::new(
            Rc::try_unwrap(parser_result)
                .map_or_else(|shared| shared.borrow().clone(), RefCell::into_inner),
        );
        self.dst().parser_result = Some(parser_result.clone());
        Ok(parser_result)
    }

    /// `getQueryCacheId()`.
    fn get_query_cache_id(&self) -> String {
        // ksort($this->hints)
        let mut hints = self.state.get_hints();
        hints.sort_by(|(a, _), (b, _)| a.cmp(b));

        let first_and_max_result = match self.state.get_hint(HINT_CUSTOM_OUTPUT_WALKER) {
            Hint::OutputWalker(output_walker_class)
                if !self.is_output_walker(output_walker_class) =>
            {
                let class = output_walker_class.name;
                crate::deprecations::trigger("https://github.com/doctrine/orm/pull/11188/", || {
                    format!(
                        "Your output walker class {class} should implement Doctrine\\ORM\\Query\\OutputWalker in order to provide a Doctrine\\ORM\\Query\\Exec\\SqlFinalizer. This also means the output walker should not use the query firstResult/maxResult values, which should be read from the query by the SqlFinalizer only."
                    )
                });
                let (first_result, max_results) = {
                    let dst = self.dst();
                    (dst.first_result, dst.max_results)
                };
                format!(
                    "&firstResult={first_result}&maxResult={}",
                    max_results.map(|m| m.to_string()).unwrap_or_default()
                )
            }
            // Assume Parser will create the SqlOutputWalker; save is_a call, which might trigger a class load
            _ => String::new(),
        };

        let em = self.get_entity_manager();
        let filters = if em.has_filters() {
            em.get_filters().get_hash()
        } else {
            String::new()
        };
        let types = format!("{:?}", self.dst().parsed_types);
        let key = format!(
            "{}{hints:?}&platform={}{filters}{first_and_max_result}&hydrationMode={}&types={types}DOCTRINE_QUERY_CACHE_SALT",
            self.get_dql().unwrap_or_default(),
            em.get_database_platform().class(),
            self.state.get_hydration_mode(),
        );
        md5_hex(key.as_bytes())
    }

    /// `is_a($outputWalkerClass, OutputWalker::class, true)`.
    fn is_output_walker(&self, output_walker_class: OutputWalkerClass) -> bool {
        let mut walker = (output_walker_class.construct)(
            self,
            Rc::new(RefCell::new(ParserResult::new())),
            QueryComponents::default(),
        );
        walker.as_output_walker().is_some()
    }

    /// Defines a cache driver to be used for caching queries.
    pub fn set_query_cache(&self, query_cache: Option<Arc<dyn CacheItemPool>>) -> &Self {
        self.dst().query_cache = query_cache;
        self
    }

    /// Defines whether the query should make use of a query cache, if available.
    pub fn use_query_cache(&self, use_query_cache: bool) -> &Self {
        self.dst().use_query_cache = use_query_cache;
        self
    }

    /// Defines how long the query cache will be active before expire.
    pub fn set_query_cache_lifetime(&self, time_to_live: Option<i64>) -> &Self {
        self.dst().query_cache_ttl = time_to_live;
        self
    }

    /// Retrieves the lifetime of resultset cache.
    pub fn get_query_cache_lifetime(&self) -> Option<i64> {
        self.dst().query_cache_ttl
    }

    /// Defines if the query cache is active or not.
    pub fn expire_query_cache(&self, expire: bool) -> &Self {
        self.dst().expire_query_cache = expire;
        self
    }

    /// Retrieves if the query cache is active or not.
    pub fn get_expire_query_cache(&self) -> bool {
        self.dst().expire_query_cache
    }

    /// `evictEntityCacheRegion()`.
    fn evict_entity_cache_region(&self) -> Result<(), Thrown> {
        let ast = Parser::new(self)?.get_ast()?;
        let class_name = match ast {
            crate::query::ast::Statement::Select(_) => {
                return Err(query_exception::new(
                    "The hint \"HINT_CACHE_EVICT\" is not valid for select statements.",
                ));
            }
            crate::query::ast::Statement::Delete(delete) => {
                delete.delete_clause.abstract_schema_name
            }
            crate::query::ast::Statement::Update(update) => {
                update.update_clause.abstract_schema_name
            }
        };
        if let Some(cache) = self.get_entity_manager().get_cache() {
            cache.evict_entity_region(&class_name)?;
        }
        Ok(())
    }

    /// Frees the resources used by the query object.
    pub fn free(&self) {
        self.state.free();
        let mut dst = self.dst();
        dst.dql = None;
        dst.state = STATE_CLEAN;
    }

    /// Sets a DQL query string.
    pub fn set_dql(&self, dql_query: &str) -> &Self {
        let mut dst = self.dst();
        dst.dql = Some(dql_query.to_owned());
        dst.state = STATE_DIRTY;
        drop(dst);
        self
    }

    /// Returns the DQL query that is represented by this query object.
    pub fn get_dql(&self) -> Option<String> {
        self.dst().dql.clone()
    }

    /// Returns the state of this query object.
    pub fn get_state(&self) -> i64 {
        self.dst().state
    }

    /// Method to check if an arbitrary piece of DQL exists.
    pub fn contains(&self, dql: &str) -> bool {
        self.get_dql()
            .unwrap_or_default()
            .to_lowercase()
            .contains(&dql.to_lowercase())
    }

    /// Sets the position of the first result to retrieve (the "offset").
    pub fn set_first_result(&self, first_result: i64) -> &Self {
        let mut dst = self.dst();
        dst.first_result = first_result;
        dst.state = STATE_DIRTY;
        drop(dst);
        self
    }

    /// Gets the position of the first result the query object was set to
    /// retrieve (the "offset").
    pub fn get_first_result(&self) -> i64 {
        self.dst().first_result
    }

    /// Sets the maximum number of results to retrieve (the "limit").
    pub fn set_max_results(&self, max_results: Option<i64>) -> &Self {
        let mut dst = self.dst();
        dst.max_results = max_results;
        dst.state = STATE_DIRTY;
        drop(dst);
        self
    }

    /// Gets the maximum number of results the query object was set to
    /// retrieve (the "limit").
    pub fn get_max_results(&self) -> Option<i64> {
        self.dst().max_results
    }

    /// Set the lock mode for this Query.
    pub fn set_lock_mode(&self, lock_mode: LockMode) -> Result<&Self, Thrown> {
        if matches!(
            lock_mode,
            LockMode::PessimisticRead | LockMode::PessimisticWrite
        ) && !self
            .get_entity_manager()
            .get_connection()
            .is_transaction_active()
        {
            return Err(crate::exception::transaction_required());
        }
        self.state
            .set_hint(HINT_LOCK_MODE, Hint::LockMode(lock_mode));
        Ok(self)
    }

    /// Get the current lock mode for this query (`None` when not set).
    pub fn get_lock_mode(&self) -> Option<LockMode> {
        match self.state.get_hint(HINT_LOCK_MODE) {
            Hint::LockMode(mode) => Some(mode),
            _ => None,
        }
    }

    /// Executes the query and returns the values it yields, row by row.
    pub async fn to_iterable(
        &self,
        parameters: Option<&PhpArray>,
        hydration_mode: i64,
    ) -> Result<Vec<Value>, Thrown> {
        self.state
            .set_hint(HINT_INTERNAL_ITERATION, Hint::Value(Value::Bool(true)));
        self.set_hydration_mode(hydration_mode);
        if let Some(parameters) = parameters.filter(|p| !p.is_empty()) {
            self.state.set_parameters_array(parameters);
        }
        // The result set mapping: the parse sets it.
        let executed = self.do_execute().await?;
        match executed {
            Executed::Rows(rows) => self.state.hydrate_iterable(rows).await,
            Executed::Count(count) => Ok(vec![Value::Int(count)]),
        }
    }

    /// `iterator_to_array($query->toIterable($parameters, $hydrationMode),
    /// true)`: the yielded values under the generator's keys (the
    /// index-by keys of an `INDEX BY` query).
    pub async fn to_iterable_array(
        &self,
        parameters: Option<&PhpArray>,
        hydration_mode: i64,
    ) -> Result<PhpArray, Thrown> {
        self.state
            .set_hint(HINT_INTERNAL_ITERATION, Hint::Value(Value::Bool(true)));
        self.set_hydration_mode(hydration_mode);
        if let Some(parameters) = parameters.filter(|p| !p.is_empty()) {
            self.state.set_parameters_array(parameters);
        }
        let executed = self.do_execute().await?;
        let yielded = match executed {
            Executed::Rows(rows) => self.state.hydrate_iterable_with_keys(rows).await?,
            Executed::Count(count) => vec![(Key::Int(0), Value::Int(count))],
        };
        let mut array = PhpArray::new();
        for (key, value) in yielded {
            array.insert(key, value);
        }
        Ok(array)
    }

    /// `_doExecute()`.
    async fn do_execute(&self) -> Result<Executed, Thrown> {
        let (mut executor, params, types) = self.prepare_execution()?;

        match self.state.get_query_cache_profile() {
            Some(profile) => executor.set_query_cache_profile(profile),
            None => executor.remove_query_cache_profile(),
        }

        self.evict_result_set_cache(
            &executor,
            &params,
            &types,
            self.get_entity_manager().get_connection().get_params(),
        )?;

        executor
            .execute(self.get_entity_manager().get_connection(), &params, &types)
            .await
    }

    /// `evictResultSetCache()`: with `expireResultCache()`, deletes the
    /// cached result of each statement.
    fn evict_result_set_cache(
        &self,
        executor: &SqlExecutor,
        sql_params: &PhpArray,
        types: &ParameterTypes,
        connection_params: &PhpArray,
    ) -> Result<(), Thrown> {
        let Some(query_cache_profile) = self.state.get_query_cache_profile() else {
            return Ok(());
        };
        if !self.state.get_expire_result_cache() {
            return Ok(());
        }

        let Some(cache) = query_cache_profile.get_result_cache() else {
            return Ok(());
        };

        let statements = match executor.get_sql_statements() {
            SqlStatements::One(sql) => vec![sql.clone()],
            SqlStatements::Many(sql) => sql.clone(),
        };

        for statement in statements {
            let (cache_key, _) = query_cache_profile.generate_cache_keys(
                &statement,
                sql_params,
                types,
                connection_params,
            );
            cache.delete_item(&cache_key)?;
        }
        Ok(())
    }

    /// The SQL of the hydration cache key (`getSQL()`, one statement).
    fn hydration_cache_sql(&self) -> Result<String, Thrown> {
        self.get_single_sql()
    }

    /// The executor and the SQL parameters of an execution.
    fn prepare_execution(&self) -> Result<(SqlExecutor, PhpArray, ParameterTypes), Thrown> {
        let parser_result = self.parse()?;
        let executor = parser_result.prepare_sql_executor(self)?;
        self.state
            .result_set_mapping_or(|| parser_result.get_result_set_mapping().clone());

        // Prepare parameters
        let param_mappings = parser_result.get_parameter_mappings().clone();
        let parameters = self.state.get_parameters();
        let param_count = i64::try_from(parameters.len()).unwrap_or(i64::MAX);
        let mapping_count = i64::try_from(param_mappings.len()).unwrap_or(i64::MAX);
        if param_count > mapping_count {
            return Err(query_exception::too_many_parameters(
                mapping_count,
                param_count,
            ));
        }
        if param_count < mapping_count {
            return Err(query_exception::too_few_parameters(
                mapping_count,
                param_count,
            ));
        }

        // evict all cache for the entity region
        if self.state.has_cache() && self.get_hint(HINT_CACHE_EVICT).is_truthy() {
            self.evict_entity_cache_region()?;
        }

        let (params, types) = self.process_parameter_mappings(&param_mappings, &parameters)?;
        Ok((executor, params, types))
    }

    /// `processParameterMappings($paramMappings)`.
    fn process_parameter_mappings(
        &self,
        param_mappings: &PhpArray,
        parameters: &[Parameter],
    ) -> Result<(PhpArray, ParameterTypes), Thrown> {
        let mut sql_params: Vec<(i64, Value)> = Vec::new();
        let mut types = ParameterTypes::new();

        for parameter in parameters {
            let key = parameter.get_name();
            let Some(Value::Array(positions)) =
                param_mappings.get(&Key::from_bytes(key.as_bytes()))
            else {
                return Err(query_exception::unknown_parameter(key));
            };
            let (value, parameter_type) = self.resolve_parameter_value(parameter)?;
            for position in positions.values() {
                let Value::Int(position) = position else {
                    continue;
                };
                types.insert(Key::Int(*position), parameter_type.clone());
                sql_params.retain(|(p, _)| p != position);
                sql_params.push((*position, value.clone()));
            }
        }

        if sql_params.len() != types.len() {
            return Err(query_exception::parameter_type_mismatch());
        }

        // ksort() and array_values()
        sql_params.sort_by_key(|(p, _)| *p);
        let params = PhpArray::list(sql_params.into_iter().map(|(_, v)| v));
        let types = types
            .into_values()
            .enumerate()
            .map(|(i, t)| (Key::Int(i64::try_from(i).unwrap_or(i64::MAX)), t))
            .collect();
        Ok((params, types))
    }

    /// `resolveParameterValue($parameter)`.
    fn resolve_parameter_value(
        &self,
        parameter: &Parameter,
    ) -> Result<(Value, php2rust_doctrine_dbal::WrapperParameterType), Thrown> {
        if parameter.type_was_specified() {
            return Ok((parameter.get_value().clone(), parameter.get_type().clone()));
        }
        let original_value = parameter.get_value();
        let mut value = original_value.clone();
        if let Value::Object(object) = original_value
            && let Some(metadata) =
                crate::collection_object::downcast::<crate::mapping::ClassMetadata>(object)
            && self.state.get_result_set_mapping().is_some_and(|rsm| {
                rsm.discriminator_parameters
                    .contains_key(&Key::from_bytes(parameter.get_name().as_bytes()))
            })
        {
            let discriminators = crate::query::utility::resolve_discriminators_for_class(
                &metadata,
                &**self.state.get_entity_manager(),
            )?;
            value = Value::Array(PhpArray::list(discriminators.keys().map(Key::to_value)));
        }
        let processed_value = self.state.process_parameter_value(&value)?;
        let type_ = if original_value.identical(&processed_value) {
            parameter.get_type().clone()
        } else {
            ParameterTypeInferer::infer_type(&processed_value)
        };
        Ok((processed_value, type_))
    }
}

impl DqlQuery for Query {
    fn get_entity_manager(&self) -> &dyn QueryContext {
        &**self.state.get_entity_manager()
    }

    fn get_dql(&self) -> String {
        self.dst().dql.clone().unwrap_or_default()
    }

    fn get_hint(&self, name: &str) -> Hint {
        self.state.get_hint(name)
    }

    fn set_hint(&self, name: &str, value: Hint) {
        self.state.set_hint(name, value);
    }

    fn get_hydration_mode(&self) -> i64 {
        self.state.get_hydration_mode()
    }

    fn get_parameter(&self, key: &str) -> Option<Parameter> {
        self.state.get_parameter(key)
    }

    fn get_first_result(&self) -> i64 {
        self.dst().first_result
    }

    fn get_max_results(&self) -> Option<i64> {
        self.dst().max_results
    }

    fn as_query(&self) -> Option<&Query> {
        Some(self)
    }
}
