//! Locking/OptimisticTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`. `#[Depends]` runs the test it depends on first and hands
//! over its entity; the entity manager is cleared, as PHPUnit's next test
//! has a new one on the same connection. `$e->getEntity()` is not kept by
//! the port's `Thrown`: the failure tests check the exception's class.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Locking/OptimisticTest.php

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

use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::{LockMode, ParameterTypes};
use php2rust_doctrine_orm::collection_object::downcast;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, new_object, obj, s, set};

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Locking\\";
const OPTIMISTIC_JOINED_PARENT: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Locking\\OptimisticJoinedParent";
const OPTIMISTIC_JOINED_CHILD: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Locking\\OptimisticJoinedChild";
const OPTIMISTIC_STANDARD: &str = "Doctrine\\Tests\\ORM\\Functional\\Locking\\OptimisticStandard";
const OPTIMISTIC_TIMESTAMP: &str = "Doctrine\\Tests\\ORM\\Functional\\Locking\\OptimisticTimestamp";
const OPTIMISTIC_LOCK_EXCEPTION: &str = "Doctrine\\ORM\\OptimisticLockException";

fn set_up() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

async fn create_schema(em: &Arc<EntityManager>) {
    orm::create_schema(
        em,
        &[
            OPTIMISTIC_JOINED_PARENT,
            OPTIMISTIC_JOINED_CHILD,
            OPTIMISTIC_STANDARD,
            OPTIMISTIC_TIMESTAMP,
        ],
    )
    .await;
}

/// `$this->_conn->executeQuery($sql, $params)`.
async fn execute(em: &EntityManager, sql: &str, params: Vec<Value>) {
    em.get_connection()
        .execute_statement(sql, &PhpArray::list(params), &ParameterTypes::new())
        .await
        .unwrap();
}

/// `$q->setParameter('id', $id)->getSingleResult()`.
async fn single_by_id(em: &Arc<EntityManager>, class: &str, id: Value) -> Object {
    let q = em.create_query(&format!("SELECT t FROM {class} t WHERE t.id = :id"));

    q.set_parameter("id", id, None);

    obj(&q.get_single_result(None).await.unwrap())
}

/// `try { flush } catch (OptimisticLockException $e) { ... }`.
fn assert_optimistic_lock_failure(result: Result<(), Thrown>) {
    if let Err(e) = result {
        assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION, "{e:?}");
    }
}

async fn joined_child_insert_sets_initial_version_value(em: &Arc<EntityManager>) -> Object {
    create_schema(em).await;
    let test = new_object(OPTIMISTIC_JOINED_CHILD);

    set(&test, "name", s("child"));
    set(&test, "whatever", s("whatever"));

    em.persist(&test).unwrap();
    em.flush().await.unwrap();

    orm::assert_loose_eq(&Value::Int(1), &get(&test, "version"));

    test
}

#[tokio::test]
async fn testJoinedChildInsertSetsInitialVersionValue() {
    let em = set_up();
    joined_child_insert_sets_initial_version_value(&em).await;
}

#[tokio::test]
async fn testJoinedChildFailureThrowsException() {
    let em = set_up();
    let child = joined_child_insert_sets_initial_version_value(&em).await;
    em.clear().unwrap();

    let test = single_by_id(
        &em,
        &format!("{NS}OptimisticJoinedChild"),
        get(&child, "id"),
    )
    .await;

    // Manually update/increment the version so we can try and save the same
    // $test and make sure the exception is thrown saying the record was
    // changed or updated since you read it
    execute(
        &em,
        "UPDATE optimistic_joined_parent SET version = ? WHERE id = ?",
        vec![Value::Int(2), get(&test, "id")],
    )
    .await;

    // Now lets change a property and try and save it again
    set(&test, "whatever", s("ok"));

    assert_optimistic_lock_failure(em.flush().await);
}

async fn joined_parent_insert_sets_initial_version_value(em: &Arc<EntityManager>) -> Object {
    create_schema(em).await;
    let test = new_object(OPTIMISTIC_JOINED_PARENT);

    set(&test, "name", s("parent"));

    em.persist(&test).unwrap();
    em.flush().await.unwrap();

    orm::assert_loose_eq(&Value::Int(1), &get(&test, "version"));

    test
}

#[tokio::test]
async fn testJoinedParentInsertSetsInitialVersionValue() {
    let em = set_up();
    joined_parent_insert_sets_initial_version_value(&em).await;
}

#[tokio::test]
async fn testJoinedParentFailureThrowsException() {
    let em = set_up();
    let parent = joined_parent_insert_sets_initial_version_value(&em).await;
    em.clear().unwrap();

    let test = single_by_id(
        &em,
        &format!("{NS}OptimisticJoinedParent"),
        get(&parent, "id"),
    )
    .await;

    // Manually update/increment the version so we can try and save the same
    // $test and make sure the exception is thrown saying the record was
    // changed or updated since you read it
    execute(
        &em,
        "UPDATE optimistic_joined_parent SET version = ? WHERE id = ?",
        vec![Value::Int(2), get(&test, "id")],
    )
    .await;

    // Now lets change a property and try and save it again
    set(&test, "name", s("WHATT???"));

    assert_optimistic_lock_failure(em.flush().await);
}

#[tokio::test]
async fn testMultipleFlushesDoIncrementalUpdates() {
    let em = set_up();
    create_schema(&em).await;

    let test = new_object(OPTIMISTIC_STANDARD);

    for i in 0..5 {
        set(&test, "name", s(&format!("test{i}")));

        em.persist(&test).unwrap();
        em.flush().await.unwrap();

        let version = get(&test, "version");
        assert!(matches!(version, Value::Int(_)), "{version:?}");
        orm::assert_loose_eq(&Value::Int(i + 1), &version);
    }
}

async fn standard_insert_sets_initial_version_value(em: &Arc<EntityManager>) -> Object {
    create_schema(em).await;

    let test = new_object(OPTIMISTIC_STANDARD);

    set(&test, "name", s("test"));

    em.persist(&test).unwrap();
    em.flush().await.unwrap();

    let version = get(&test, "version");
    assert!(matches!(version, Value::Int(_)), "{version:?}");
    orm::assert_loose_eq(&Value::Int(1), &version);

    test
}

#[tokio::test]
async fn testStandardInsertSetsInitialVersionValue() {
    let em = set_up();
    standard_insert_sets_initial_version_value(&em).await;
}

#[tokio::test]
async fn testStandardFailureThrowsException() {
    let em = set_up();
    let entity = standard_insert_sets_initial_version_value(&em).await;
    em.clear().unwrap();

    let test = single_by_id(&em, &format!("{NS}OptimisticStandard"), get(&entity, "id")).await;

    // Manually update/increment the version so we can try and save the same
    // $test and make sure the exception is thrown saying the record was
    // changed or updated since you read it
    execute(
        &em,
        "UPDATE optimistic_standard SET version = ? WHERE id = ?",
        vec![Value::Int(2), get(&test, "id")],
    )
    .await;

    // Now lets change a property and try and save it again
    set(&test, "name", s("WHATT???"));

    assert_optimistic_lock_failure(em.flush().await);
}

#[tokio::test]
async fn testLockWorksWithProxy() {
    let em = set_up();
    create_schema(&em).await;
    let test = new_object(OPTIMISTIC_STANDARD);
    set(&test, "name", s("test"));

    em.persist(&test).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let proxy = em
        .get_reference(OPTIMISTIC_STANDARD, get(&test, "id"))
        .await
        .unwrap()
        .unwrap();

    em.lock(&proxy, LockMode::Optimistic, Some(&Value::Int(1)))
        .await
        .unwrap();
}

/// `$value instanceof DateTime`.
fn date_time(value: &Value) -> Arc<DateTime> {
    match value {
        Value::Object(object) => downcast::<DateTime>(object).expect("a DateTime"),
        other => panic!("not a DateTime: {other:?}"),
    }
}

async fn optimistic_timestamp_sets_default_value(em: &Arc<EntityManager>) -> Object {
    create_schema(em).await;
    let test = new_object(OPTIMISTIC_TIMESTAMP);

    set(&test, "name", s("Testing"));

    assert!(get(&test, "version").is_null(), "Pre-Condition");

    em.persist(&test).unwrap();
    em.flush().await.unwrap();

    date_time(&get(&test, "version"));

    test
}

#[tokio::test]
async fn testOptimisticTimestampSetsDefaultValue() {
    let em = set_up();
    optimistic_timestamp_sets_default_value(&em).await;
}

#[tokio::test]
async fn testOptimisticTimestampFailureThrowsException() {
    let em = set_up();
    let entity = optimistic_timestamp_sets_default_value(&em).await;
    em.clear().unwrap();

    let test = single_by_id(&em, &format!("{NS}OptimisticTimestamp"), get(&entity, "id")).await;

    let version = date_time(&get(&test, "version"));

    // Manually increment the version datetime column
    let format = em.get_platform().get_date_time_format_string();

    let incremented =
        DateTimeImmutable::from_timestamp(version.get().timestamp() + 3600, 0, TimeZone::Utc)
            .format(&format);
    execute(
        &em,
        "UPDATE optimistic_timestamp SET version = ? WHERE id = ?",
        vec![s(&incremented), get(&test, "id")],
    )
    .await;

    // Try and update the record and it should throw an exception
    set(&test, "name", s("Testing again"));

    let caught_exception = em.flush().await.err();

    let e = caught_exception.expect("No OptimisticLockingException was thrown");
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION, "{e:?}");
}

#[tokio::test]
async fn testOptimisticTimestampLockFailureThrowsException() {
    let em = set_up();
    let entity = optimistic_timestamp_sets_default_value(&em).await;
    em.clear().unwrap();

    let test = single_by_id(&em, &format!("{NS}OptimisticTimestamp"), get(&entity, "id")).await;

    let version = date_time(&get(&test, "version"));

    // Try to lock the record with an older timestamp and it should throw an exception
    let expected_version_expired = DateTime::object(DateTimeImmutable::from_timestamp(
        version.get().timestamp() - 3600,
        0,
        TimeZone::Utc,
    ));

    let caught_exception = em
        .lock(&test, LockMode::Optimistic, Some(&expected_version_expired))
        .await
        .err();

    let e = caught_exception.expect("No OptimisticLockingException was thrown");
    assert_eq!(e.class, OPTIMISTIC_LOCK_EXCEPTION, "{e:?}");
}
