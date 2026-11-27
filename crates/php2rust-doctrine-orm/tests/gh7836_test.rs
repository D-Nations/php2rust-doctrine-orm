//! Ticket/GH7836Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7836Test.php

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

/// `GH7836ParentEntity::addChild($position, $name)`: `$this->children[] = ...`
/// makes the unset property an array.
fn add_child(parent: &Object, position: i64, name: &str) {
    // new GH7836ChildEntity($this, $position, $name)
    let child = new("GH7836ChildEntity");
    set_obj(&child, "parent", parent);
    set(&child, "position", Value::Int(position));
    set(&child, "name", s(name));

    let mut children = match get(parent, "children") {
        Value::Array(children) => children,
        _ => PhpArray::new(),
    };
    children.push(Value::Object(child)).unwrap();
    set(parent, "children", Value::Array(children));
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["GH7836ParentEntity", "GH7836ChildEntity"]).await;

    let parent = new("GH7836ParentEntity");
    add_child(&parent, 100, "foo");
    add_child(&parent, 100, "bar");
    add_child(&parent, 200, "baz");

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

/// The `(position, name)` of the first three matched children.
async fn children(em: &EntityManager, criteria: &Criteria) -> Vec<(Value, Value)> {
    let parent = find(em, "GH7836ParentEntity", Value::Int(1)).await.unwrap();
    let children = coll::matching(&get(&parent, "children"), criteria)
        .await
        .unwrap();
    let mut result = Vec::new();
    for i in 0..3 {
        let child = obj(&coll::get(&children, i).await.unwrap());
        result.push((get(&child, "position"), get(&child, "name")));
    }
    result
}

fn assert_child(child: &(Value, Value), position: i64, name: &str) {
    assert!(Value::Int(position).identical(&child.0), "{child:?}");
    assert!(s(name).identical(&child.1), "{child:?}");
}

#[tokio::test]
async fn testMatchingRespectsCollectionOrdering() {
    let em = set_up().await;

    // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
    let children = children(&em, &Criteria::create()).await;

    assert_child(&children[0], 100, "bar");
    assert_child(&children[1], 100, "foo");
    assert_child(&children[2], 200, "baz");
}

#[tokio::test]
async fn testMatchingOverrulesCollectionOrdering() {
    let em = set_up().await;

    let mut criteria = Criteria::create();
    criteria.order_by([
        ("position", SortDirection::Descending),
        ("name", SortDirection::Ascending),
    ]);
    let children = children(&em, &criteria).await;

    assert_child(&children[0], 200, "baz");
    assert_child(&children[1], 100, "bar");
    assert_child(&children[2], 100, "foo");
}

#[tokio::test]
async fn testMatchingKeepsOrderOfCriteriaOrderingKeys() {
    let em = set_up().await;

    let mut criteria = Criteria::create();
    criteria.order_by([
        ("name", SortDirection::Ascending),
        ("position", SortDirection::Ascending),
    ]);
    let children = children(&em, &criteria).await;

    assert_child(&children[0], 100, "bar");
    assert_child(&children[1], 200, "baz");
    assert_child(&children[2], 100, "foo");
}
