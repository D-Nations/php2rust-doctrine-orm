//! The `PaginatorTest` and `OffsetPaginatorTest` connection: a driver
//! connection stub that records the parameters of each query it runs
//! (`executeQuery()`'s `willReturnCallback()`) and gives the rows queued
//! for it, in order (none once the queue is empty), in place of the
//! upstream hydrator stub's `hydrateAll()` results.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/PaginatorTest.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use php_value::{Key, PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::driver::api::ExceptionConverter;
use php2rust_doctrine_dbal::driver::{
    BoxFuture, Connection as DriverConnection, Driver, PlatformBox, Result as DriverResult,
    Statement, pdo_sqlite,
};
use php2rust_doctrine_dbal::{Connection, Exception, ParameterType, ServerVersionProvider};

/// What the stub records and gives.
#[derive(Default)]
pub struct Recorder {
    /// The bound values of each query run, in order.
    pub received_params: Vec<Vec<Value>>,
    /// The rows for the next queries.
    pub rows: VecDeque<Vec<PhpArray>>,
}

pub type SharedRecorder = Arc<Mutex<Recorder>>;

struct RecordingDriver {
    inner: pdo_sqlite::Driver,
    recorder: SharedRecorder,
}

struct RecordingConnection {
    recorder: SharedRecorder,
}

struct RecordingStatement {
    recorder: SharedRecorder,
    bound: Vec<(i64, Value)>,
}

struct RowsResult {
    rows: VecDeque<PhpArray>,
    columns: usize,
}

impl Recorder {
    fn run(&mut self, params: Vec<Value>) -> RowsResult {
        self.received_params.push(params);
        let rows: VecDeque<PhpArray> = self.rows.pop_front().unwrap_or_default().into();
        let columns = rows.front().map_or(0, PhpArray::len);
        RowsResult { rows, columns }
    }
}

impl Driver for RecordingDriver {
    fn class(&self) -> &'static str {
        "Doctrine\\DBAL\\Driver"
    }

    fn connect<'a>(
        &'a self,
        _params: &'a PhpArray,
    ) -> BoxFuture<'a, Result<Arc<dyn DriverConnection>, Exception>> {
        let recorder = Arc::clone(&self.recorder);
        Box::pin(async move {
            Ok(Arc::new(RecordingConnection { recorder }) as Arc<dyn DriverConnection>)
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

impl DriverConnection for RecordingConnection {
    fn prepare<'a>(
        &'a self,
        _sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn Statement>, Exception>> {
        let recorder = Arc::clone(&self.recorder);
        Box::pin(async move {
            Ok(Box::new(RecordingStatement {
                recorder,
                bound: Vec::new(),
            }) as Box<dyn Statement>)
        })
    }

    fn query<'a>(
        &'a self,
        _sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn DriverResult>, Exception>> {
        let result = self.recorder.lock().unwrap().run(Vec::new());
        Box::pin(async move { Ok(Box::new(result) as Box<dyn DriverResult>) })
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

    /// A stub's native connection: an object of no class.
    fn get_native_connection(&self) -> php2rust_doctrine_dbal::driver::NativeConnection {
        std::sync::Arc::new(())
    }
}

impl Statement for RecordingStatement {
    fn bind_value(
        &mut self,
        param: &Key,
        value: Value,
        _type: ParameterType,
    ) -> Result<(), Exception> {
        let position = match param {
            Key::Int(i) => *i,
            Key::Str(_) => i64::try_from(self.bound.len()).unwrap() + 1,
        };
        self.bound.retain(|(p, _)| *p != position);
        self.bound.push((position, value));
        Ok(())
    }

    fn execute(&mut self) -> BoxFuture<'_, Result<Box<dyn DriverResult>, Exception>> {
        self.bound.sort_by_key(|(p, _)| *p);
        let params = self.bound.iter().map(|(_, v)| v.clone()).collect();
        let result = self.recorder.lock().unwrap().run(params);
        Box::pin(async move { Ok(Box::new(result) as Box<dyn DriverResult>) })
    }
}

impl DriverResult for RowsResult {
    fn fetch_numeric(&mut self) -> Result<Option<PhpArray>, Exception> {
        Ok(self
            .rows
            .pop_front()
            .map(|row| PhpArray::list(row.values().cloned())))
    }

    fn fetch_associative(&mut self) -> Result<Option<PhpArray>, Exception> {
        Ok(self.rows.pop_front())
    }

    fn row_count(&self) -> Result<i64, Exception> {
        Ok(0)
    }

    fn column_count(&self) -> Result<usize, Exception> {
        Ok(self.columns)
    }

    fn free(&mut self) {}

    fn class(&self) -> &'static str {
        "Doctrine\\DBAL\\Driver\\Result"
    }
}

/// The recording connection and its recorder.
pub fn recording_connection() -> (Arc<Connection>, SharedRecorder) {
    let recorder = SharedRecorder::default();
    let connection = Arc::new(Connection::new(
        PhpArray::new(),
        Arc::new(RecordingDriver {
            inner: pdo_sqlite::Driver::new(),
            recorder: Arc::clone(&recorder),
        }),
        None,
    ));
    (connection, recorder)
}

/// A row of one column.
pub fn row(column: &str, value: impl Into<Value>) -> PhpArray {
    let mut row = PhpArray::new();
    row.insert(column, value.into());
    row
}
