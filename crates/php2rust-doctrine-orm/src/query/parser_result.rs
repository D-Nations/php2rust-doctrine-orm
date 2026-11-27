//! `Doctrine\ORM\Query\ParserResult`: the result of parsing a DQL query,
//! the SQL finalizer (or legacy executor), the result set mapping and the
//! parameter mappings.
//!
//! Upstream: doctrine/orm src/Query/ParserResult.php

use std::sync::Arc;

use php_value::{Key, PhpArray, Thrown, Value};

use crate::deprecations;
use crate::query::ResultSetMapping;
use crate::query::context::DqlQuery;
use crate::query::exec::{SqlExecutor, SqlFinalizer};
use crate::query::query_exception;

const SQL_EXECUTOR_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/11188";

fn array(value: Option<&Value>) -> PhpArray {
    match value {
        Some(Value::Array(a)) => a.clone(),
        _ => PhpArray::new(),
    }
}

/// `ParserResult`. A clone shares the finalizer (PHP's query cache
/// keeps a copy of the whole result).
#[derive(Clone)]
pub struct ParserResult {
    sql_executor: Option<SqlExecutor>,
    sql_finalizer: Option<Arc<dyn SqlFinalizer>>,
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

impl Default for ParserResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserResult {
    pub fn new() -> Self {
        Self {
            sql_executor: None,
            sql_finalizer: None,
            result_set_mapping: ResultSetMapping::new(),
            parameter_mappings: PhpArray::new(),
        }
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
        self.sql_finalizer = Some(Arc::from(finalizer));
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
