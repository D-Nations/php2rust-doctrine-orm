//! Ticket/GH7629Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7629Test.php

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

use php_value::Value;

use ticket::*;

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    let em = ticket::set_up(&["GH7629Entity"]).await;
    em.persist(&new("GH7629Entity")).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

#[tokio::test]
async fn testClearScheduledForSynchronizationWhenCommitEmpty() {
    let em = set_up().await;
    let entity = find(&em, "GH7629Entity", Value::Int(1)).await.unwrap();

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert!(
        !em.get_unit_of_work()
            .is_scheduled_for_dirty_check(&entity)
            .unwrap()
    );
}

#[tokio::test]
async fn testPersistAfterRemoveSchedulesForSynchronization() {
    let em = set_up().await;
    let entity = find(&em, "GH7629Entity", Value::Int(1)).await.unwrap();

    em.remove(&entity).await.unwrap();

    em.persist(&entity).unwrap();

    assert!(
        em.get_unit_of_work()
            .is_scheduled_for_dirty_check(&entity)
            .unwrap()
    );
}
