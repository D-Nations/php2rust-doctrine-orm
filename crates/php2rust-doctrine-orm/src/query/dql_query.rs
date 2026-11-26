//! `Doctrine\ORM\Query`: a DQL query.
//!
//! The query is parsed each time its SQL is needed (PHP keeps the parser
//! result of a clean query, and caches it in the query cache, which is
//! not ported): the parser result and its walkers are not `Send`, so they
//! never live across an `await`. The query keeps PHP's result set mapping
//! once a parse or `setResultSetMapping()` set it.
//!
//! Upstream: doctrine/orm src/Query.php

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::{LockMode, ParameterTypes};

use crate::abstract_query::{Executed, QueryState, abstract_query_methods};
use crate::entity_manager::EntityManager;
use crate::query::context::{
    DqlQuery, HINT_INTERNAL_ITERATION, HINT_LOCK_MODE, Hint, QueryContext,
};
use crate::query::exec::{SqlExecutor, SqlStatements};
use crate::query::{Parameter, ParameterTypeInferer, Parser, query_exception};

pub use crate::query::context::{
    HINT_CACHE_ENABLED, HINT_CACHE_EVICT, HINT_CUSTOM_OUTPUT_WALKER, HINT_CUSTOM_TREE_WALKERS,
    HINT_FORCE_PARTIAL_LOAD, HINT_INCLUDE_META_COLUMNS, HINT_READ_ONLY, HINT_REFRESH,
    HINT_REFRESH_ENTITY,
};

/// `Query::STATE_CLEAN`.
pub const STATE_CLEAN: i64 = 1;
/// `Query::STATE_DIRTY`.
pub const STATE_DIRTY: i64 = 2;

struct DqlState {
    dql: Option<String>,
    first_result: i64,
    max_results: Option<i64>,
    state: i64,
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
            dql: Mutex::new(DqlState {
                dql: None,
                first_result: 0,
                max_results: None,
                state: STATE_CLEAN,
            }),
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
        Self {
            state: self.state.clone_state(),
            dql: Mutex::new(DqlState {
                dql: dst.dql.clone(),
                first_result: dst.first_result,
                max_results: dst.max_results,
                state: STATE_DIRTY,
            }),
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
        let parser_result = Parser::new(self)?.parse()?;
        let parser_result = parser_result.borrow();
        self.dst().state = STATE_CLEAN;
        self.state
            .result_set_mapping_or(|| parser_result.get_result_set_mapping().clone());
        parser_result.prepare_sql_executor(self)
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

    /// `_doExecute()`.
    async fn do_execute(&self) -> Result<Executed, Thrown> {
        let (executor, params, types) = self.prepare_execution()?;
        executor
            .execute(self.get_entity_manager().get_connection(), &params, &types)
            .await
    }

    /// The executor and the SQL parameters of an execution.
    fn prepare_execution(&self) -> Result<(SqlExecutor, PhpArray, ParameterTypes), Thrown> {
        let parser_result = Parser::new(self)?.parse()?;
        let parser_result = parser_result.borrow();
        self.dst().state = STATE_CLEAN;
        let executor = parser_result.prepare_sql_executor(self)?;
        self.state
            .result_set_mapping_or(|| parser_result.get_result_set_mapping().clone());

        // Prepare parameters
        let param_mappings = parser_result.get_parameter_mappings().clone();
        drop(parser_result);
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
        let processed_value = self.state.process_parameter_value(original_value)?;
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
