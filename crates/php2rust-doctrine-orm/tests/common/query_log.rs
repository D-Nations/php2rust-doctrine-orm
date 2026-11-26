//! `Doctrine\Tests\DbalExtensions\QueryLog`: a driver that records the
//! statements the connection runs (a prepared statement when it is
//! prepared, which the connection does once per execution), not the
//! transaction commands.
//!
//! Upstream: doctrine/orm tests/Tests/DbalExtensions/QueryLog.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use std::sync::{Arc, Mutex, PoisonError};

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::driver::api::ExceptionConverter;
use php2rust_doctrine_dbal::driver::{
    BoxFuture, Connection as DriverConnection, Driver, PlatformBox, Result as DriverResult, Statement,
    pdo_sqlite,
};
use php2rust_doctrine_dbal::{Connection, Exception, ServerVersionProvider};

/// The recorded statements, and whether recording is on.
#[derive(Debug, Default)]
pub struct QueryLog {
    queries: Mutex<(bool, Vec<String>)>,
}

impl QueryLog {
    /// `reset()->enable()`.
    pub fn reset_and_enable(&self) {
        *self.queries.lock().unwrap_or_else(PoisonError::into_inner) = (true, Vec::new());
    }

    fn log(&self, sql: &str) {
        let mut queries = self.queries.lock().unwrap_or_else(PoisonError::into_inner);
        if queries.0 {
            queries.1.push(sql.to_owned());
        }
    }

    /// The statements recorded since `reset_and_enable()`.
    pub fn queries(&self) -> Vec<String> {
        self.queries.lock().unwrap_or_else(PoisonError::into_inner).1.clone()
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
        self.log.log(sql);
        self.inner.prepare(sql)
    }

    fn query<'a>(&'a self, sql: &'a str) -> BoxFuture<'a, Result<Box<dyn DriverResult>, Exception>> {
        self.log.log(sql);
        self.inner.query(sql)
    }

    fn quote(&self, value: &[u8]) -> PhpString {
        self.inner.quote(value)
    }

    fn exec<'a>(&'a self, sql: &'a str) -> BoxFuture<'a, Result<i64, Exception>> {
        self.log.log(sql);
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
    (Arc::new(Connection::new(params, Arc::new(driver), None)), log)
}

/// `setUp()` on a connection whose statements are logged.
pub async fn set_up_logged(
    classes: &[&str],
) -> (Arc<php2rust_doctrine_orm::entity_manager::EntityManager>, Arc<QueryLog>) {
    let (conn, log) = logging_connection();
    let em = super::orm::entity_manager(conn, None);
    super::orm::create_schema(&em, classes).await;
    (em, log)
}
