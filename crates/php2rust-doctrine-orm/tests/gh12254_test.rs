//! Ticket/GH12254Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12254Test.php

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

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use ticket::*;

/// `setUp()`, then `$this->_em = $this->getEntityManager()` on the same
/// connection.
async fn set_up_fresh() -> Arc<EntityManager> {
    let em = set_up(&["GH12254EntityA"]).await;

    em.persist(&new("GH12254EntityA")).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // pretend we are starting afresh
    orm::entity_manager(em.get_connection().clone(), None)
}

/// `findBy([$field => []])`.
async fn find_by_empty(em: &Arc<EntityManager>, field: &str) -> usize {
    let mut criteria = PhpArray::new();
    criteria.insert(field, Value::Array(PhpArray::new()));
    em.get_repository(&class("GH12254EntityA"))
        .unwrap()
        .find_by(&criteria, None, None, None)
        .await
        .unwrap()
        .len()
}

#[tokio::test]
async fn testFindByEmptyArrayShouldReturnEmptyArray() {
    let em = set_up_fresh().await;
    assert_eq!(0, find_by_empty(&em, "id").await);
}

#[tokio::test]
async fn testFindByInNullableField() {
    let em = set_up_fresh().await;
    assert_eq!(0, find_by_empty(&em, "name").await);
}
