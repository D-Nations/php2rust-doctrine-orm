//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ReadOnlyTest.php
//!
//! On `SQLite` (`tests/common/orm.rs`). `ReadOnlyEntity` is a recorded
//! line of `tests/data/models.json`.

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::{HINT_READ_ONLY, Hint};

use orm::{get, new_object, s, set};

const READ_ONLY_ENTITY: &str = "Doctrine\\Tests\\ORM\\Functional\\ReadOnlyEntity";

/// `setUp()`.
async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[READ_ONLY_ENTITY]).await
}

/// `new ReadOnlyEntity($name, $numericValue)`.
fn read_only_entity(name: &str, numeric_value: i64) -> Object {
    let entity = new_object(READ_ONLY_ENTITY);
    set(&entity, "name", s(name));
    set(&entity, "numericValue", Value::Int(numeric_value));
    entity
}

/// `SELECT u FROM ReadOnlyEntity u WHERE u.id = ?1` with the read-only
/// hint, `getSingleResult()`.
async fn single_result_with_read_only_hint(
    em: &Arc<EntityManager>,
    id: Value,
    read_only: bool,
) -> Object {
    let dql = format!("SELECT u FROM {READ_ONLY_ENTITY} u WHERE u.id = ?1");

    let query = em.create_query(&dql);
    query.set_parameter(1, id, None);
    query.set_hint(HINT_READ_ONLY, Hint::Value(Value::Bool(read_only)));

    orm::obj(&query.get_single_result(None).await.unwrap())
}

#[tokio::test]
async fn testReadOnlyEntityNeverChangeTracked() {
    let em = set_up().await;
    let read_only = read_only_entity("Test1", 1234);
    em.persist(&read_only).unwrap();
    em.flush().await.unwrap();

    set(&read_only, "name", s("Test2"));
    set(&read_only, "numericValue", Value::Int(4321));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let db_read_only = em
        .find(READ_ONLY_ENTITY, get(&read_only, "id"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(s("Test1"), get(&db_read_only, "name"));
    assert_eq!(Value::Int(1234), get(&db_read_only, "numericValue"));
}

#[tokio::test]
async fn testClearReadOnly() {
    let em = set_up().await;
    let read_only = read_only_entity("Test1", 1234);
    em.persist(&read_only).unwrap();
    em.flush().await.unwrap();
    em.get_unit_of_work().mark_read_only(&read_only).unwrap();

    em.clear().unwrap();

    assert!(!em.get_unit_of_work().is_read_only(&read_only));
}

#[tokio::test]
async fn testClearEntitiesReadOnly() {
    let em = set_up().await;
    let read_only = read_only_entity("Test1", 1234);
    em.persist(&read_only).unwrap();
    em.flush().await.unwrap();
    em.get_unit_of_work().mark_read_only(&read_only).unwrap();

    em.clear().unwrap();

    assert!(!em.get_unit_of_work().is_read_only(&read_only));
}

#[tokio::test]
async fn testReadOnlyQueryHint() {
    let em = set_up().await;
    let user = read_only_entity("beberlei", 1234);

    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = single_result_with_read_only_hint(&em, get(&user, "id"), true).await;

    assert!(em.get_unit_of_work().is_read_only(&user));
}

#[tokio::test]
async fn testNotReadOnlyQueryHint() {
    let em = set_up().await;
    let user = read_only_entity("beberlei", 1234);

    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = single_result_with_read_only_hint(&em, get(&user, "id"), false).await;

    assert!(!em.get_unit_of_work().is_read_only(&user));
}

#[tokio::test]
async fn testNotReadOnlyIfObjectWasProxyBefore() {
    let em = set_up().await;
    let user = read_only_entity("beberlei", 1234);

    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em
        .get_reference(READ_ONLY_ENTITY, get(&user, "id"))
        .await
        .unwrap()
        .unwrap();

    let user = single_result_with_read_only_hint(&em, get(&user, "id"), true).await;

    assert!(!em.get_unit_of_work().is_read_only(&user));
}

#[tokio::test]
async fn testNotReadOnlyIfObjectWasKnownBefore() {
    let em = set_up().await;
    let user = read_only_entity("beberlei", 1234);

    em.persist(&user).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let _user_into_identity_map = em.find(READ_ONLY_ENTITY, get(&user, "id")).await.unwrap();

    let user = single_result_with_read_only_hint(&em, get(&user, "id"), true).await;

    assert!(!em.get_unit_of_work().is_read_only(&user));
}
