//! Ticket/DDC3634Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; a wrapping driver stands in for the DBAL middleware).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3634Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::{Arc, Mutex};

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::driver::{
    self, BoxFuture, PlatformBox, Result as DriverResult, Statement, api::ExceptionConverter,
};
use php2rust_doctrine_dbal::{Connection, Exception, ServerVersionProvider};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::get;
use ticket::*;

/// `LastInsertIdMocker`.
#[derive(Default)]
struct LastInsertIdMocker {
    mocked_id: Mutex<Option<String>>,
}

/// `LastInsertIdMockConnection`: the wrapped connection, with the mocked
/// `lastInsertId()`.
struct LastInsertIdMockConnection {
    wrapped: Arc<dyn driver::Connection>,
    id_mocker: Arc<LastInsertIdMocker>,
}

impl driver::Connection for LastInsertIdMockConnection {
    fn prepare<'a>(&'a self, sql: &'a str) -> BoxFuture<'a, Result<Box<dyn Statement>, Exception>> {
        self.wrapped.prepare(sql)
    }

    fn query<'a>(
        &'a self,
        sql: &'a str,
    ) -> BoxFuture<'a, Result<Box<dyn DriverResult>, Exception>> {
        self.wrapped.query(sql)
    }

    fn quote(&self, value: &[u8]) -> PhpString {
        self.wrapped.quote(value)
    }

    fn exec<'a>(&'a self, sql: &'a str) -> BoxFuture<'a, Result<i64, Exception>> {
        self.wrapped.exec(sql)
    }

    fn last_insert_id(&self) -> BoxFuture<'_, Result<Value, Exception>> {
        let mocked = self.id_mocker.mocked_id.lock().unwrap().clone();
        match mocked {
            Some(id) => Box::pin(async move { Ok(Value::string(id)) }),
            None => self.wrapped.last_insert_id(),
        }
    }

    fn begin_transaction(&self) -> BoxFuture<'_, Result<(), Exception>> {
        self.wrapped.begin_transaction()
    }

    fn commit(&self) -> BoxFuture<'_, Result<(), Exception>> {
        self.wrapped.commit()
    }

    fn roll_back(&self) -> BoxFuture<'_, Result<(), Exception>> {
        self.wrapped.roll_back()
    }

    fn get_server_version(&self) -> BoxFuture<'_, Result<String, Exception>> {
        self.wrapped.get_server_version()
    }
}

/// `LastInsertIdMockDriver`: the wrapped driver, connecting through
/// `LastInsertIdMockConnection`.
struct LastInsertIdMockDriver {
    wrapped: Arc<dyn driver::Driver>,
    id_mocker: Arc<LastInsertIdMocker>,
}

impl driver::Driver for LastInsertIdMockDriver {
    fn class(&self) -> &'static str {
        "LastInsertIdMockDriver"
    }

    fn connect<'a>(
        &'a self,
        params: &'a PhpArray,
    ) -> BoxFuture<'a, Result<Arc<dyn driver::Connection>, Exception>> {
        Box::pin(async move {
            let wrapped = self.wrapped.connect(params).await?;
            let connection: Arc<dyn driver::Connection> = Arc::new(LastInsertIdMockConnection {
                wrapped,
                id_mocker: Arc::clone(&self.id_mocker),
            });
            Ok(connection)
        })
    }

    fn get_database_platform<'a>(
        &'a self,
        version_provider: &'a dyn ServerVersionProvider,
    ) -> BoxFuture<'a, Result<PlatformBox, Exception>> {
        self.wrapped.get_database_platform(version_provider)
    }

    fn get_exception_converter(&self) -> Arc<dyn ExceptionConverter> {
        self.wrapped.get_exception_converter()
    }
}

const MODELS: [&str; 3] = [
    "DDC3634Entity",
    "DDC3634JTIBaseEntity",
    "DDC3634JTIChildEntity",
];

/// `setUp()`: `$config->setMiddlewares([new LastInsertIdMockMiddleware(...)])`
/// is a connection over `LastInsertIdMockDriver`.
async fn set_up() -> (Arc<EntityManager>, Arc<LastInsertIdMocker>) {
    let id_mocker = Arc::new(LastInsertIdMocker::default());
    let plain = orm::connection();
    let driver = Arc::new(LastInsertIdMockDriver {
        wrapped: Arc::clone(plain.get_driver()),
        id_mocker: Arc::clone(&id_mocker),
    });
    let conn = Arc::new(Connection::new(plain.get_params().clone(), driver, None));
    let em = orm::entity_manager(conn, None);

    let metadata = em.get_class_metadata(&class("DDC3634Entity")).unwrap();
    assert!(
        metadata
            .read()
            .id_generator
            .as_ref()
            .is_some_and(php2rust_doctrine_orm::id::IdGenerator::is_post_insert_generator),
        "Need a post-insert ID generator in order to make this test work correctly"
    );

    let classes: Vec<String> = MODELS.iter().map(|short| class(short)).collect();
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    orm::create_schema(&em, &classes).await;
    (em, id_mocker)
}

#[tokio::test]
async fn testSavesVeryLargeIntegerAutoGeneratedValue() {
    let (em, id_mocker) = set_up().await;
    let very_large_id = format!("{}{}", i64::MAX, i64::MAX);
    *id_mocker.mocked_id.lock().unwrap() = Some(very_large_id.clone());

    let entity = new("DDC3634Entity");

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert_eq!(Value::string(very_large_id), get(&entity, "id"));
}

#[tokio::test]
async fn testSavesIntegerAutoGeneratedValue() {
    let (em, _id_mocker) = set_up().await;
    let entity = new("DDC3634Entity");

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert!(!matches!(get(&entity, "id"), Value::Null));
}

#[tokio::test]
async fn testSavesIntegerAutoGeneratedValueWithJoinedInheritance() {
    let (em, _id_mocker) = set_up().await;
    let entity = new("DDC3634JTIChildEntity");

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert!(!matches!(get(&entity, "id"), Value::Null));
}
