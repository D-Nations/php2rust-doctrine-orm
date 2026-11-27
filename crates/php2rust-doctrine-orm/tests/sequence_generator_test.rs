//! Port of tests/Tests/ORM/Id/SequenceGeneratorTest.php. PHPUnit's mock
//! of `Connection::fetchOne()` is a driver here whose queries give the
//! test's current `$i` and are counted; the platform stub's
//! `getSequenceNextValSQL()` gives `''`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Id/SequenceGeneratorTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/platform.rs"]
mod platform;

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::driver::api::ExceptionConverter;
use php2rust_doctrine_dbal::driver::{
    BoxFuture, Connection as DriverConnection, Driver, PlatformBox, Result as DriverResult,
    Statement, pdo_sqlite,
};
use php2rust_doctrine_dbal::{Connection, Exception, ParameterType, ServerVersionProvider};
use php2rust_doctrine_orm::id::SequenceGenerator;

use platform::TestPlatform;

/// `$i` of the test and the number of `fetchOne()` calls.
#[derive(Default)]
struct Shared {
    i: AtomicI64,
    fetch_one_calls: AtomicI64,
}

struct MockDriver {
    shared: Arc<Shared>,
    inner: pdo_sqlite::Driver,
}

struct MockConnection {
    shared: Arc<Shared>,
}

/// The one-row result of `fetchOne()`.
struct OneValue(Option<Value>);

impl Driver for MockDriver {
    fn class(&self) -> &'static str {
        "Doctrine\\DBAL\\Driver"
    }

    fn connect<'a>(
        &'a self,
        _params: &'a PhpArray,
    ) -> BoxFuture<'a, Result<Arc<dyn DriverConnection>, Exception>> {
        let shared = self.shared.clone();
        Box::pin(
            async move { Ok(Arc::new(MockConnection { shared }) as Arc<dyn DriverConnection>) },
        )
    }

    fn get_database_platform<'a>(
        &'a self,
        _version_provider: &'a dyn ServerVersionProvider,
    ) -> BoxFuture<'a, Result<PlatformBox, Exception>> {
        Box::pin(async {
            Ok(
                Box::new(TestPlatform::new(platform::ABSTRACT_PLATFORM).sequence_next_val_sql(""))
                    as PlatformBox,
            )
        })
    }

    fn get_exception_converter(&self) -> Arc<dyn ExceptionConverter> {
        self.inner.get_exception_converter()
    }
}

impl MockConnection {
    /// `->willReturnCallback(static function () use (&$i) { ... })`.
    fn fetch_one(&self) -> Box<dyn DriverResult> {
        self.shared.fetch_one_calls.fetch_add(1, Ordering::SeqCst);
        let i = self.shared.i.load(Ordering::SeqCst);
        assert_eq!(0, i % 10);
        Box::new(OneValue(Some(Value::Int(i))))
    }
}

struct MockStatement(Option<Box<dyn DriverResult>>);

impl Statement for MockStatement {
    fn bind_value(
        &mut self,
        _param: &php_value::Key,
        _value: Value,
        _type: ParameterType,
    ) -> Result<(), Exception> {
        Ok(())
    }

    fn execute(&mut self) -> BoxFuture<'_, Result<Box<dyn DriverResult>, Exception>> {
        let result = self.0.take();
        Box::pin(async move {
            result.ok_or_else(|| Exception::new(Exception::UNSUPPORTED, "executed twice"))
        })
    }
}

impl DriverConnection for MockConnection {
    fn prepare<'a>(
        &'a self,
        _sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn Statement>, Exception>> {
        let result = self.fetch_one();
        Box::pin(async move { Ok(Box::new(MockStatement(Some(result))) as Box<dyn Statement>) })
    }

    fn query<'a>(
        &'a self,
        _sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn DriverResult>, Exception>> {
        let result = self.fetch_one();
        Box::pin(async move { Ok(result) })
    }

    fn quote(&self, value: &[u8]) -> PhpString {
        PhpString::from(value)
    }

    fn exec<'a>(&'a self, _sql: &'a str) -> BoxFuture<'a, Result<i64, Exception>> {
        Box::pin(async { Ok(0) })
    }

    fn last_insert_id(&self) -> BoxFuture<'_, Result<Value, Exception>> {
        Box::pin(async { Ok(Value::Int(0)) })
    }

    fn begin_transaction(&self) -> BoxFuture<'_, Result<(), Exception>> {
        Box::pin(async { Ok(()) })
    }

    fn commit(&self) -> BoxFuture<'_, Result<(), Exception>> {
        Box::pin(async { Ok(()) })
    }

    fn roll_back(&self) -> BoxFuture<'_, Result<(), Exception>> {
        Box::pin(async { Ok(()) })
    }

    fn get_server_version(&self) -> BoxFuture<'_, Result<String, Exception>> {
        Box::pin(async { Ok("3.0.0".to_owned()) })
    }

    /// A mock's native connection: an object of no class.
    fn get_native_connection(&self) -> php2rust_doctrine_dbal::driver::NativeConnection {
        std::sync::Arc::new(())
    }
}

impl DriverResult for OneValue {
    fn fetch_numeric(&mut self) -> Result<Option<PhpArray>, Exception> {
        Ok(self.0.take().map(|value| PhpArray::list([value])))
    }

    fn fetch_associative(&mut self) -> Result<Option<PhpArray>, Exception> {
        Ok(self.0.take().map(|value| {
            let mut row = PhpArray::new();
            row.insert("nextval", value);
            row
        }))
    }

    fn row_count(&self) -> Result<i64, Exception> {
        Ok(1)
    }

    fn column_count(&self) -> Result<usize, Exception> {
        Ok(1)
    }

    fn free(&mut self) {
        self.0 = None;
    }

    fn class(&self) -> &'static str {
        "Doctrine\\DBAL\\Driver\\Result"
    }
}

#[tokio::test]
async fn testGeneration() {
    let sequence_generator = SequenceGenerator::new("seq", 10);

    let shared = Arc::new(Shared::default());
    let connection = Arc::new(Connection::new(
        PhpArray::new(),
        Arc::new(MockDriver {
            shared: shared.clone(),
            inner: pdo_sqlite::Driver::new(),
        }),
        None,
    ));

    let entity_manager = orm::entity_manager(connection, None);

    for i in 0..42_i64 {
        shared.i.store(i, Ordering::SeqCst);
        let id = sequence_generator
            .generate_id(&entity_manager, None)
            .await
            .unwrap();

        assert_eq!(i, id);
        assert_eq!(
            Some(i / 10 * 10 + 10),
            sequence_generator.get_current_max_value()
        );
        assert_eq!(i + 1, sequence_generator.get_next_value());
    }

    // Sequence values should be generated once per ten identifiers
    assert_eq!(5, shared.fetch_one_calls.load(Ordering::SeqCst));
}
