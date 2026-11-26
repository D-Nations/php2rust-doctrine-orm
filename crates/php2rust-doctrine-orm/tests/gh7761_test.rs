//! Ticket/GH7761Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7761Test.php

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
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// `new GH7761Entity()`.
fn entity() -> php_value::Object {
    let entity = new("GH7761Entity");
    set(
        &entity,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    entity
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["GH7761Entity", "GH7761ChildEntity"]).await;

    let parent = entity();
    let child = new("GH7761ChildEntity");
    coll::add(&get(&parent, "children"), Value::Object(child.clone())).unwrap();

    em.persist(&parent).unwrap();
    em.persist(&child).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

#[tokio::test]
async fn testCollectionClearDoesNotClearIfNotPersisted() {
    let em = set_up().await;
    let entity_1 = find(&em, "GH7761Entity", Value::Int(1)).await.unwrap();
    coll::clear(&get(&entity_1, "children")).await.unwrap();
    em.persist(&entity()).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let entity_1 = find(&em, "GH7761Entity", Value::Int(1)).await.unwrap();
    assert_eq!(1, count_of(&entity_1, "children").await);
}

#[tokio::test]
async fn testCollectionClearDoesClearIfPersisted() {
    let em = set_up().await;
    let entity_1 = find(&em, "GH7761Entity", Value::Int(1)).await.unwrap();
    coll::clear(&get(&entity_1, "children")).await.unwrap();
    em.persist(&entity_1).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let entity_1 = find(&em, "GH7761Entity", Value::Int(1)).await.unwrap();
    assert_eq!(0, count_of(&entity_1, "children").await);
}
