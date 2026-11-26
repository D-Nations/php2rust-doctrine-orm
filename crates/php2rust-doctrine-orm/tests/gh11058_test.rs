//! Ticket/GH11058Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11058Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// `GH11058Parent::addChild($child)` (with `GH11058Child::setParent()`).
async fn add_child(parent: &Object, child: &Object) {
    let children = get(parent, "children");
    if !coll::contains(&children, &Value::Object(child.clone()))
        .await
        .unwrap()
    {
        coll::add(&children, Value::Object(child.clone())).unwrap();
        set_obj(child, "parent", parent);
    }
}

async fn create_parent_with_two_child_entities() -> (Arc<EntityManager>, [Object; 3]) {
    let em = ticket::set_up(&["GH11058Parent", "GH11058Child"]).await;
    let parent = new("GH11058Parent");
    set(
        &parent,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    let child1 = new("GH11058Child");
    let child2 = new("GH11058Child");

    add_child(&parent, &child1).await;
    add_child(&parent, &child2).await;

    (em, [parent, child1, child2])
}

fn id(entity: &Object) -> i64 {
    match get(entity, "id") {
        Value::Int(id) => id,
        id => panic!("{id:?} is not an id"),
    }
}

async fn persist_in_order(em: &EntityManager, entities: [&Object; 3]) {
    for entity in entities {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
}

#[tokio::test]
async fn testChildrenInsertedInOrderOfPersistCalls1WhenParentPersistedLast() {
    let (em, [parent, child1, child2]) = create_parent_with_two_child_entities().await;

    persist_in_order(&em, [&child1, &child2, &parent]).await;

    assert!(id(&child1) < id(&child2));
}

#[tokio::test]
async fn testChildrenInsertedInOrderOfPersistCalls2WhenParentPersistedLast() {
    let (em, [parent, child1, child2]) = create_parent_with_two_child_entities().await;

    persist_in_order(&em, [&child2, &child1, &parent]).await;

    assert!(id(&child2) < id(&child1));
}

#[tokio::test]
async fn testChildrenInsertedInOrderOfPersistCalls1WhenParentPersistedFirst() {
    let (em, [parent, child1, child2]) = create_parent_with_two_child_entities().await;

    persist_in_order(&em, [&parent, &child1, &child2]).await;

    assert!(id(&child1) < id(&child2));
}

#[tokio::test]
async fn testChildrenInsertedInOrderOfPersistCalls2WhenParentPersistedFirst() {
    let (em, [parent, child1, child2]) = create_parent_with_two_child_entities().await;

    persist_in_order(&em, [&parent, &child2, &child1]).await;

    assert!(id(&child2) < id(&child1));
}
