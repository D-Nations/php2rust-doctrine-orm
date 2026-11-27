//! Ticket/GH7512Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7512Test.php

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

use php_value::{PhpArray, Value};

use ticket::*;

#[tokio::test]
async fn testFindEntityByAssociationPropertyJoinedChildWithClearMetadata() {
    let em = ticket::set_up(&["GH7512EntityA", "GH7512EntityB", "GH7512EntityC"]).await;
    em.persist(&new("GH7512EntityA")).unwrap();
    em.persist(&new("GH7512EntityC")).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // pretend we are starting afresh
    let em = orm::entity_manager(em.get_connection().clone(), None);
    let mut criteria = PhpArray::new();
    criteria.insert("entityA", Value::Object(new("GH7512EntityB")));
    let result = em
        .get_repository(&class("GH7512EntityC"))
        .unwrap()
        .find_by(&criteria, None, None, None)
        .await
        .unwrap();
    assert!(result.is_empty());
}
