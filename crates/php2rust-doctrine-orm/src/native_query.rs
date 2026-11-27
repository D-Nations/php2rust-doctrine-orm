//! `Doctrine\ORM\NativeQuery`: a native SQL query, hydrated with a result
//! set mapping.
//!
//! Upstream: doctrine/orm src/NativeQuery.php

use std::sync::{Arc, Mutex, PoisonError};

use php_value::{Key, PhpArray, Thrown};
use php2rust_doctrine_dbal::ParameterTypes;

use crate::abstract_query::{Executed, QueryState, abstract_query_methods};
use crate::entity_manager::EntityManager;
use crate::query::ParameterTypeInferer;

/// Represents a native SQL query.
pub struct NativeQuery {
    state: QueryState,
    sql: Mutex<String>,
}

impl std::fmt::Debug for NativeQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeQuery")
            .field("sql", &self.get_sql())
            .finish_non_exhaustive()
    }
}

impl NativeQuery {
    /// `new NativeQuery($em)`.
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            state: QueryState::new(em),
            sql: Mutex::new(String::new()),
        }
    }

    /// The `AbstractQuery` state.
    pub fn state(&self) -> &QueryState {
        &self.state
    }

    abstract_query_methods!();

    pub fn set_sql(&self, sql: &str) -> &Self {
        sql.clone_into(&mut self.sql.lock().unwrap_or_else(PoisonError::into_inner));
        self
    }

    pub fn get_sql(&self) -> String {
        self.sql
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// `getResultSetMapping()`.
    fn get_result_set_mapping(&self) -> Result<crate::query::ResultSetMapping, Thrown> {
        self.state
            .get_result_set_mapping()
            .ok_or_else(|| Thrown::new("LogicException", "Uninitialized result set mapping.", 0))
    }

    /// `getHash()`.
    fn get_hash(&self) -> Result<String, Thrown> {
        self.state.get_hash(&self.get_sql())
    }

    /// `_doExecute()`.
    async fn do_execute(&self) -> Result<Executed, Thrown> {
        let mut parameters = PhpArray::new();
        let mut types = ParameterTypes::new();

        for parameter in self.state.get_parameters() {
            let name = Key::from_bytes(parameter.get_name().as_bytes());
            if parameter.type_was_specified() {
                parameters.insert(name.clone(), parameter.get_value().clone());
                types.insert(name, parameter.get_type().clone());
                continue;
            }
            let value = self.state.process_parameter_value(parameter.get_value())?;
            let parameter_type = if parameter.get_value().identical(&value) {
                parameter.get_type().clone()
            } else {
                ParameterTypeInferer::infer_type(&value)
            };
            parameters.insert(name.clone(), value);
            types.insert(name, parameter_type);
        }

        if matches!(parameters.keys().next(), Some(Key::Int(_))) {
            parameters.ksort();
            parameters = PhpArray::list(parameters.values().cloned());
            types = types
                .into_values()
                .enumerate()
                .map(|(i, t)| (Key::Int(i64::try_from(i).unwrap_or(i64::MAX)), t))
                .collect();
        }

        let rows = self
            .get_entity_manager()
            .get_connection()
            .execute_query_with_profile(
                &self.get_sql(),
                &parameters,
                &types,
                self.state.get_query_cache_profile().as_ref(),
            )
            .await
            .and_then(|mut result| result.fetch_all_associative())
            .map_err(|e| crate::exception::from_dbal(&e))?;
        Ok(Executed::Rows(rows))
    }

    /// The SQL of the hydration cache key (`getSQL()`).
    #[allow(clippy::unnecessary_wraps, reason = "the DQL query's can fail")]
    fn hydration_cache_sql(&self) -> Result<String, Thrown> {
        Ok(self.get_sql())
    }
}
