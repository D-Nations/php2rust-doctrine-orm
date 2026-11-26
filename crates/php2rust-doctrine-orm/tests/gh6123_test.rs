//! Ticket/GH6123Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6123Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_REMOVED};

use orm::*;
use ticket::*;

/// `loadEntityFromDatabase($id)`.
async fn load_entity_from_database(em: &Arc<EntityManager>, id: Value) -> Value {
    let mut qb = em.create_query_builder();
    qb.select(vec![s("e")])
        .unwrap()
        .from(&class("GH6123Entity"), "e", None)
        .unwrap()
        .r#where(vec![s("e.id = :id")])
        .unwrap()
        .set_parameter("id", id, None);
    qb.get_query()
        .unwrap()
        .get_one_or_null_result(None)
        .await
        .unwrap()
}

#[tokio::test]
async fn testLoadingRemovedEntityFromDatabaseDoesNotCreateNewManagedEntityInstance() {
    let em = ticket::set_up(&["GH6123Entity"]).await;
    let entity = new("GH6123Entity");
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    let uow = em.get_unit_of_work();
    assert_eq!(STATE_MANAGED, uow.get_entity_state(&entity).await.unwrap());
    assert!(!uow.is_scheduled_for_delete(&entity));

    em.remove(&entity).await.unwrap();

    let fresh_entity = load_entity_from_database(&em, get(&entity, "id")).await;
    assert!(same_value(&fresh_entity, &entity));
    let fresh_entity = obj(&fresh_entity);

    assert_eq!(
        STATE_REMOVED,
        uow.get_entity_state(&fresh_entity).await.unwrap()
    );
    assert!(uow.is_scheduled_for_delete(&fresh_entity));
}

#[tokio::test]
async fn testRemovedEntityCanBePersistedAgain() {
    let em = ticket::set_up(&["GH6123Entity"]).await;
    let entity = new("GH6123Entity");
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    let uow = em.get_unit_of_work();
    em.remove(&entity).await.unwrap();
    assert_eq!(STATE_REMOVED, uow.get_entity_state(&entity).await.unwrap());
    assert!(uow.is_scheduled_for_delete(&entity));

    load_entity_from_database(&em, get(&entity, "id")).await;

    em.persist(&entity).unwrap();
    assert_eq!(STATE_MANAGED, uow.get_entity_state(&entity).await.unwrap());
    assert!(!uow.is_scheduled_for_delete(&entity));

    em.flush().await.unwrap();
}
