//! The `UnitOfWorkTest` connection: a driver connection stub whose
//! statements run nothing and give no rows, and whose `lastInsertId()`
//! gives 1, 2, 3, ... (PHPUnit's `createStub()` with
//! `willReturnOnConsecutiveCalls()`). A failing commit or rollback is a
//! driver exception.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/UnitOfWorkTest.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::driver::api::ExceptionConverter;
use php2rust_doctrine_dbal::driver::{
    BoxFuture, Connection as DriverConnection, Driver, PlatformBox, Result as DriverResult,
    Statement, pdo_sqlite,
};
use php2rust_doctrine_dbal::{Connection, Exception, ParameterType, ServerVersionProvider};

/// What fails.
#[derive(Debug, Clone, Copy, Default)]
pub struct Failures {
    pub commit: Option<&'static str>,
    pub roll_back: Option<&'static str>,
}

struct StubDriver {
    inner: pdo_sqlite::Driver,
    failures: Failures,
}

struct StubConnection {
    last_insert_id: AtomicI64,
    failures: Failures,
}

struct StubStatement;

struct StubResult;

fn failure(message: &str) -> Exception {
    Exception::driver("Doctrine\\DBAL\\Driver\\Exception", message, None, 0)
}

impl Driver for StubDriver {
    fn class(&self) -> &'static str {
        "Doctrine\\DBAL\\Driver"
    }

    fn connect<'a>(
        &'a self,
        _params: &'a PhpArray,
    ) -> BoxFuture<'a, Result<Arc<dyn DriverConnection>, Exception>> {
        let failures = self.failures;
        Box::pin(async move {
            Ok(Arc::new(StubConnection {
                last_insert_id: AtomicI64::new(0),
                failures,
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

impl DriverConnection for StubConnection {
    fn prepare<'a>(
        &'a self,
        _sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn Statement>, Exception>> {
        Box::pin(async { Ok(Box::new(StubStatement) as Box<dyn Statement>) })
    }

    fn query<'a>(
        &'a self,
        _sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn DriverResult>, Exception>> {
        Box::pin(async { Ok(Box::new(StubResult) as Box<dyn DriverResult>) })
    }

    fn quote(&self, value: &[u8]) -> PhpString {
        PhpString::from(value)
    }

    fn exec<'a>(&'a self, _sql: &'a str) -> BoxFuture<'a, Result<i64, Exception>> {
        Box::pin(async { Ok(0) })
    }

    fn last_insert_id(&self) -> BoxFuture<'_, Result<Value, Exception>> {
        let id = self.last_insert_id.fetch_add(1, Ordering::SeqCst) + 1;
        Box::pin(async move { Ok(Value::Int(id)) })
    }

    fn begin_transaction(&self) -> BoxFuture<'_, Result<(), Exception>> {
        Box::pin(async { Ok(()) })
    }

    fn commit(&self) -> BoxFuture<'_, Result<(), Exception>> {
        let fails = self.failures.commit;
        Box::pin(async move { fails.map_or(Ok(()), |message| Err(failure(message))) })
    }

    fn roll_back(&self) -> BoxFuture<'_, Result<(), Exception>> {
        let fails = self.failures.roll_back;
        Box::pin(async move { fails.map_or(Ok(()), |message| Err(failure(message))) })
    }

    fn get_server_version(&self) -> BoxFuture<'_, Result<String, Exception>> {
        Box::pin(async { Ok("3.0.0".to_owned()) })
    }

    /// A stub's native connection: an object of no class.
    fn get_native_connection(&self) -> php2rust_doctrine_dbal::driver::NativeConnection {
        std::sync::Arc::new(())
    }
}

impl Statement for StubStatement {
    fn bind_value(
        &mut self,
        _param: &php_value::Key,
        _value: Value,
        _type: ParameterType,
    ) -> Result<(), Exception> {
        Ok(())
    }

    fn execute(&mut self) -> BoxFuture<'_, Result<Box<dyn DriverResult>, Exception>> {
        Box::pin(async { Ok(Box::new(StubResult) as Box<dyn DriverResult>) })
    }
}

impl DriverResult for StubResult {
    fn fetch_numeric(&mut self) -> Result<Option<PhpArray>, Exception> {
        Ok(None)
    }

    fn fetch_associative(&mut self) -> Result<Option<PhpArray>, Exception> {
        Ok(None)
    }

    fn row_count(&self) -> Result<i64, Exception> {
        Ok(0)
    }

    fn column_count(&self) -> Result<usize, Exception> {
        Ok(0)
    }

    fn free(&mut self) {}

    fn class(&self) -> &'static str {
        "Doctrine\\DBAL\\Driver\\Result"
    }
}

/// The stubbed connection.
pub fn stub_connection(failures: Failures) -> Arc<Connection> {
    Arc::new(Connection::new(
        PhpArray::new(),
        Arc::new(StubDriver {
            inner: pdo_sqlite::Driver::new(),
            failures,
        }),
        None,
    ))
}
