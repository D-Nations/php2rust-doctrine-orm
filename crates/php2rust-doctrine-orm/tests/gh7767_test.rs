//! Ticket/GH7767Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7767Test.php

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
use php2rust_doctrine_collections::{Criteria, SortDirection};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// `GH7767ParentEntity::addChild($position)`: `$this->children[] = ...`
/// makes the unset property an array.
fn add_child(parent: &Object, position: i64) {
    // new GH7767ChildEntity($this, $position)
    let child = new("GH7767ChildEntity");
    set_obj(&child, "parent", parent);
    set(&child, "position", Value::Int(position));

    let mut children = match get(parent, "children") {
        Value::Array(children) => children,
        _ => PhpArray::new(),
    };
    children.push(Value::Object(child)).unwrap();
    set(parent, "children", Value::Array(children));
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["GH7767ParentEntity", "GH7767ChildEntity"]).await;

    let parent = new("GH7767ParentEntity");
    add_child(&parent, 200);
    add_child(&parent, 100);
    add_child(&parent, 300);

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

async fn positions(em: &EntityManager, criteria: &Criteria) -> Vec<Value> {
    let parent = find(em, "GH7767ParentEntity", Value::Int(1)).await.unwrap();
    let children = coll::matching(&get(&parent, "children"), criteria)
        .await
        .unwrap();
    let mut positions = Vec::new();
    for i in 0..3 {
        let child = coll::get(&children, i).await.unwrap();
        positions.push(get(&obj(&child), "position"));
    }
    positions
}

#[tokio::test]
async fn testMatchingRespectsCollectionOrdering() {
    let em = set_up().await;

    // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
    let children = positions(&em, &Criteria::create()).await;

    assert_loose_eq(&Value::Int(100), &children[0]);
    assert_loose_eq(&Value::Int(200), &children[1]);
    assert_loose_eq(&Value::Int(300), &children[2]);
}

#[tokio::test]
async fn testMatchingOverrulesCollectionOrdering() {
    let em = set_up().await;

    // collections 3 (3.1 and later): `SortDirection::Descending`.
    let mut criteria = Criteria::create();
    criteria.order_by([("position", SortDirection::Descending)]);
    let children = positions(&em, &criteria).await;

    assert_loose_eq(&Value::Int(300), &children[0]);
    assert_loose_eq(&Value::Int(200), &children[1]);
    assert_loose_eq(&Value::Int(100), &children[2]);
}
