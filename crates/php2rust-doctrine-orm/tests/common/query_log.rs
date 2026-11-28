//! `Doctrine\Tests\DbalExtensions\QueryLog`: a driver that records the
//! statements the connection runs (a prepared statement when it is
//! prepared, which the connection does once per execution), not the
//! transaction commands.
//!
//! Upstream: doctrine/orm tests/Tests/DbalExtensions/QueryLog.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::{Arc, Mutex, PoisonError};

use php_value::{Key, PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::driver::api::ExceptionConverter;
use php2rust_doctrine_dbal::driver::{
    BoxFuture, Connection as DriverConnection, Driver, PlatformBox, Result as DriverResult,
    Statement, pdo_sqlite,
};
use php2rust_doctrine_dbal::{Connection, Exception, ParameterType, ServerVersionProvider};

/// The recorded statements, and whether recording is on.
#[derive(Debug, Default)]
pub struct QueryLog {
    queries: Mutex<(bool, Vec<(String, PhpArray)>)>,
    /// The parameter types bound to each statement, by its position.
    types: Mutex<Vec<(usize, Key, ParameterType)>>,
}

impl QueryLog {
    /// `reset()->enable()`.
    pub fn reset_and_enable(&self) {
        *self.queries.lock().unwrap_or_else(PoisonError::into_inner) = (true, Vec::new());
        self.types
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// Records a statement: its position in the log.
    fn log(&self, sql: &str) -> Option<usize> {
        let mut queries = self.queries.lock().unwrap_or_else(PoisonError::into_inner);
        if queries.0 {
            queries.1.push((sql.to_owned(), PhpArray::new()));
            return Some(queries.1.len() - 1);
        }
        None
    }

    fn bind_type(&self, position: usize, param: &Key, r#type: ParameterType) {
        self.types
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((position, param.clone(), r#type));
    }

    /// `getLastLoggedQuery()['types']`: the types bound to the last
    /// statement, in binding order.
    pub fn last_types(&self) -> Vec<(Key, ParameterType)> {
        let last = self
            .queries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
            .len()
            .checked_sub(1);
        self.types
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter(|(position, _, _)| Some(*position) == last)
            .map(|(_, key, r#type)| (key.clone(), *r#type))
            .collect()
    }

    fn bind(&self, position: usize, param: &Key, value: &Value) {
        let mut queries = self.queries.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, params)) = queries.1.get_mut(position) {
            params.insert(param.clone(), value.clone());
        }
    }

    /// `getLastLoggedQuery()['params']`: the values bound to the last
    /// statement.
    pub fn last_params(&self) -> PhpArray {
        self.queries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
            .last()
            .map_or_default(|(_, params)| params.clone())
    }

    /// The statements recorded since `reset_and_enable()`, with their
    /// bound values and types, in binding order.
    pub fn entries(&self) -> Vec<(String, PhpArray, Vec<ParameterType>)> {
        let types = self.types.lock().unwrap_or_else(PoisonError::into_inner);
        self.queries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
            .iter()
            .enumerate()
            .map(|(position, (sql, params))| {
                let bound = types
                    .iter()
                    .filter(|(at, _, _)| *at == position)
                    .map(|(_, _, r#type)| *r#type)
                    .collect();
                (sql.clone(), params.clone(), bound)
            })
            .collect()
    }

    /// The statements recorded since `reset_and_enable()`.
    pub fn queries(&self) -> Vec<String> {
        self.queries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
            .iter()
            .map(|(sql, _)| sql.clone())
            .collect()
    }
}

/// A statement whose bound values go to the log.
struct LoggingStatement {
    inner: Box<dyn Statement>,
    log: Arc<QueryLog>,
    position: Option<usize>,
}

impl Statement for LoggingStatement {
    fn bind_value(
        &mut self,
        param: &Key,
        value: Value,
        r#type: ParameterType,
    ) -> Result<(), Exception> {
        if let Some(position) = self.position {
            self.log.bind(position, param, &value);
            self.log.bind_type(position, param, r#type);
        }
        self.inner.bind_value(param, value, r#type)
    }

    fn execute(&mut self) -> BoxFuture<'_, Result<Box<dyn DriverResult>, Exception>> {
        self.inner.execute()
    }
}

struct LoggingDriver {
    inner: pdo_sqlite::Driver,
    log: Arc<QueryLog>,
}

struct LoggingConnection {
    inner: Arc<dyn DriverConnection>,
    log: Arc<QueryLog>,
}

impl Driver for LoggingDriver {
    fn class(&self) -> &str {
        self.inner.class()
    }

    fn connect<'a>(
        &'a self,
        params: &'a PhpArray,
    ) -> BoxFuture<'a, Result<Arc<dyn DriverConnection>, Exception>> {
        Box::pin(async move {
            let inner = self.inner.connect(params).await?;
            Ok(Arc::new(LoggingConnection {
                inner,
                log: self.log.clone(),
            }) as Arc<dyn DriverConnection>)
        })
    }

    fn get_database_platform<'a>(
        &'a self,
        version_provider: &'a dyn ServerVersionProvider,
    ) -> BoxFuture<'a, Result<PlatformBox, Exception>> {
        self.inner.get_database_platform(version_provider)
    }

    fn get_exception_converter(&self) -> Arc<dyn ExceptionConverter> {
        self.inner.get_exception_converter()
    }
}

impl DriverConnection for LoggingConnection {
    fn prepare<'a>(&'a self, sql: &'a str) -> BoxFuture<'a, Result<Box<dyn Statement>, Exception>> {
        Box::pin(async move {
            let position = self.log.log(sql);
            let inner = self.inner.prepare(sql).await?;
            Ok(Box::new(LoggingStatement {
                inner,
                log: self.log.clone(),
                position,
            }) as Box<dyn Statement>)
        })
    }

    fn query<'a>(
        &'a self,
        sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn DriverResult>, Exception>> {
        let _ = self.log.log(sql);
        self.inner.query(sql)
    }

    fn quote(&self, value: &[u8]) -> PhpString {
        self.inner.quote(value)
    }

    fn exec<'a>(&'a self, sql: &'a str) -> BoxFuture<'a, Result<i64, Exception>> {
        let _ = self.log.log(sql);
        self.inner.exec(sql)
    }

    fn last_insert_id(&self) -> BoxFuture<'_, Result<Value, Exception>> {
        self.inner.last_insert_id()
    }

    fn begin_transaction(&self) -> BoxFuture<'_, Result<(), Exception>> {
        self.inner.begin_transaction()
    }

    fn commit(&self) -> BoxFuture<'_, Result<(), Exception>> {
        self.inner.commit()
    }

    fn roll_back(&self) -> BoxFuture<'_, Result<(), Exception>> {
        self.inner.roll_back()
    }

    fn get_server_version(&self) -> BoxFuture<'_, Result<String, Exception>> {
        self.inner.get_server_version()
    }

    fn get_native_connection(&self) -> php2rust_doctrine_dbal::driver::NativeConnection {
        self.inner.get_native_connection()
    }
}

/// An in-memory `SQLite` connection whose statements go to the log.
pub fn logging_connection() -> (Arc<Connection>, Arc<QueryLog>) {
    let log = Arc::new(QueryLog::default());
    let mut params = PhpArray::new();
    params.insert("memory", Value::Bool(true));
    let driver = LoggingDriver {
        inner: pdo_sqlite::Driver::new(),
        log: log.clone(),
    };
    (
        Arc::new(Connection::new(params, Arc::new(driver), None)),
        log,
    )
}

/// `setUp()` on a connection whose statements are logged.
pub async fn set_up_logged(
    classes: &[&str],
) -> (
    Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
    Arc<QueryLog>,
) {
    let (conn, log) = logging_connection();
    let em = super::orm::entity_manager(conn, None);
    super::orm::create_schema(&em, classes).await;
    (em, log)
}
