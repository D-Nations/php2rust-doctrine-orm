//! Ticket/DDC599Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC599Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    ticket::set_up(&["DDC599Item", "DDC599Subitem", "DDC599Child"]).await
}

/// `new DDC599Subitem()`.
fn subitem() -> Object {
    let item = new("DDC599Subitem");
    set(
        &item,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    item
}

#[tokio::test]
async fn testCascadeRemoveOnInheritanceHierarchy() {
    let em = set_up().await;

    let item = subitem();
    set(&item, "elem", s("foo"));
    let child = new("DDC599Child");
    set_obj(&child, "parent", &item);
    coll::add(&get(&item, "children"), Value::Object(child.clone())).unwrap();
    em.persist(&item).unwrap();
    em.persist(&child).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let item = find(&em, "DDC599Item", get(&item, "id")).await.unwrap();

    em.remove(&item).await.unwrap();
    em.flush().await.unwrap(); // Should not fail

    assert!(!em.contains(&item).unwrap());
    let children = get(&item, "children");
    assert!(
        !em.contains(&obj(&coll::get(&children, 0).await.unwrap()))
            .unwrap()
    );

    em.clear().unwrap();

    let item2 = subitem();
    set(&item2, "elem", s("bar"));
    em.persist(&item2).unwrap();
    em.flush().await.unwrap();

    let child2 = new("DDC599Child");
    set_obj(&child2, "parent", &item2);
    coll::add(&get(&item2, "children"), Value::Object(child2.clone())).unwrap();
    em.persist(&child2).unwrap();
    em.flush().await.unwrap();

    em.remove(&item2).await.unwrap();
    em.flush().await.unwrap(); // should not fail

    assert!(!em.contains(&item).unwrap());
    let children = get(&item, "children");
    assert!(
        !em.contains(&obj(&coll::get(&children, 0).await.unwrap()))
            .unwrap()
    );
}

#[tokio::test]
async fn testCascadeRemoveOnChildren() {
    let em = set_up().await;

    let class = em.get_class_metadata(&class("DDC599Subitem")).unwrap();
    let class = class.read();
    assert!(class.association_mappings.get("children").is_some());
    assert!(
        class
            .association_mappings
            .get("children")
            .unwrap()
            .is_cascade_remove()
    );
}
