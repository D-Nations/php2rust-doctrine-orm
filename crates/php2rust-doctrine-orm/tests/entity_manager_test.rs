//! Upstream: doctrine/orm tests/Tests/ORM/EntityManagerTest.php
//!
//! The `EntityManagerMock` is an entity manager on an in-memory `SQLite`
//! connection. `assertInstanceOf()` of a getter's class is its type. The
//! data provider rows run in one test each; `new stdClass()` is a generic
//! object. A closure that throws a `TypeError` returns it.
//!
//! `testItDoesNotAttemptToRollbackIfNoTransactionIsActive`'s connection
//! subclass is a driver connection stub (`tests/common/stub_connection.rs`)
//! whose commit fails: DBAL's `commit()` then ends the transaction, so
//! `isTransactionActive()` is false, and a rollback would fail with the
//! `Assert::fail()` message, which would replace the commit's.
//!
//! Pending: `testLazyGhostEntityManager` (a lazy ghost of the entity
//! manager through reflection) and
//! `testItPreservesTheOriginalExceptionOnRollbackFailure` (`Thrown` has
//! no previous exception).

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
#[path = "common/stub_connection.rs"]
mod stub_connection;

use std::sync::Arc;

use php_value::object::GenericObject;
use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::ENTITY_MANAGER_CLOSED;
use php2rust_doctrine_orm::query::ResultSetMapping;

use orm::CMS_USER;

fn set_up() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

#[test]
fn testIsOpen() {
    let entity_manager = set_up();
    assert!(entity_manager.is_open());
    entity_manager.close();
    assert!(!entity_manager.is_open());
}

#[test]
fn testGetConnection() {
    let entity_manager = set_up();
    let _: &Arc<php2rust_doctrine_dbal::Connection> = entity_manager.get_connection();
}

#[test]
fn testGetMetadataFactory() {
    let entity_manager = set_up();
    let _: &Arc<php2rust_doctrine_orm::mapping::ClassMetadataFactory> =
        entity_manager.get_metadata_factory();
}

#[test]
fn testGetProxyFactory() {
    let entity_manager = set_up();
    let _: php2rust_doctrine_orm::proxy::ProxyFactory = entity_manager.get_proxy_factory();
}

#[test]
fn testGetConfiguration() {
    let entity_manager = set_up();
    let _: &php2rust_doctrine_orm::configuration::Configuration =
        entity_manager.get_configuration();
}

#[test]
fn testGetUnitOfWork() {
    let entity_manager = set_up();
    let _: &php2rust_doctrine_orm::unit_of_work::UnitOfWork = entity_manager.get_unit_of_work();
}

#[test]
fn testGetEventManager() {
    let entity_manager = set_up();
    let _: php2rust_doctrine_event_manager::EventManager = entity_manager.get_event_manager();
}

#[test]
fn testCreateNativeQuery() {
    let entity_manager = set_up();
    let rsm = ResultSetMapping::new();
    let query = entity_manager
        .create_native_query("SELECT foo", rsm)
        .unwrap();
    assert_eq!("SELECT foo", query.get_sql());
}

#[test]
fn testCreateQueryBuilder() {
    let entity_manager = set_up();
    let _: php2rust_doctrine_orm::query_builder::QueryBuilder =
        entity_manager.create_query_builder();
}

#[test]
fn testCreateQueryBuilderAliasValid() {
    let entity_manager = set_up();
    let mut q = entity_manager.create_query_builder();
    q.select(vec![Value::string("u")])
        .unwrap()
        .from(CMS_USER, "u", None)
        .unwrap();
    let q2 = q.clone();

    assert_eq!(
        Some("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u".to_owned()),
        q.get_query().unwrap().get_dql()
    );
    assert_eq!(
        Some("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u".to_owned()),
        q2.get_query().unwrap().get_dql()
    );

    let q3 = q.clone();

    assert_eq!(
        Some("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u".to_owned()),
        q3.get_query().unwrap().get_dql()
    );
}

#[test]
fn testCreateQueryDqlIsOptional() {
    let entity_manager = set_up();
    let _: php2rust_doctrine_orm::query::Query = entity_manager.create_query("");
}

#[test]
fn testCreateQuery() {
    let entity_manager = set_up();
    let q = entity_manager.create_query("SELECT 1");
    assert_eq!(Some("SELECT 1".to_owned()), q.get_dql());
}

/// `new stdClass()`.
fn std_class() -> Object {
    Arc::new(GenericObject::new("stdClass", PhpArray::new()))
}

#[tokio::test]
async fn testAffectedByErrorIfClosedException() {
    for method_name in ["flush", "persist", "remove", "refresh"] {
        let entity_manager = set_up();
        entity_manager.close();
        let object = std_class();
        let result = match method_name {
            "flush" => entity_manager.flush().await,
            "persist" => entity_manager.persist(&object),
            "remove" => entity_manager.remove(&object).await,
            _ => entity_manager.refresh(&object).await,
        };
        let e = result.unwrap_err();
        assert_eq!(ENTITY_MANAGER_CLOSED, e.class, "{method_name}");
        assert!(
            String::from_utf8_lossy(e.message.as_bytes()).contains("closed"),
            "{method_name}"
        );
    }
}

#[tokio::test]
async fn testWrapInTransactionAcceptsReturn() {
    let expected_values = [
        Value::Array(PhpArray::new()),
        Value::Array(PhpArray::list([Value::Int(1)])),
        Value::Int(0),
        Value::Float(100.5),
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::string("foo"),
    ];
    for expected_value in expected_values {
        let entity_manager = set_up();
        let value = expected_value.clone();
        let r#return = entity_manager
            .wrap_in_transaction(|_em| async move { Ok(value) })
            .await
            .unwrap();
        assert!(expected_value.identical(&r#return), "{expected_value:?}");
    }
}

#[tokio::test]
async fn testWrapInTransactionReThrowsThrowables() {
    let entity_manager = set_up();
    let result: Result<(), Thrown> = entity_manager
        .wrap_in_transaction(|_em| async move {
            // this only serves as an IIFE that throws a `TypeError`
            Err(Thrown::new(
                "TypeError",
                "{closure}(): Argument #1 ($value) must be of type array, null given",
                0,
            ))
        })
        .await;
    let e = result.expect_err("TypeError expected to be thrown");
    assert_eq!("TypeError", e.class);
    assert!(!entity_manager.is_open());
}

#[tokio::test]
async fn testItDoesNotAttemptToRollbackIfNoTransactionIsActive() {
    let entity_manager = orm::entity_manager(
        stub_connection::stub_connection(stub_connection::Failures {
            commit: Some("Commit exception that happens after doing the actual commit"),
            roll_back: Some("Should not attempt to rollback if no transaction is active"),
        }),
        None,
    );

    let e = entity_manager
        .wrap_in_transaction(|_em| async move { Ok(()) })
        .await
        .unwrap_err();
    assert!(
        String::from_utf8_lossy(e.message.as_bytes()).contains("Commit exception"),
        "{e:?}"
    );
}
