//! Ticket/DDC992Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC992Test.php

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
    ticket::set_up(&["DDC992Role", "DDC992Parent", "DDC992Child"]).await
}

/// `new DDC992Role()`.
fn role(name: &str) -> Object {
    let role = new("DDC992Role");
    set(
        &role,
        "extends",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &role,
        "extendedBy",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&role, "name", s(name));
    role
}

#[tokio::test]
async fn testIssue() {
    let em = set_up().await;

    let role = role("Parent");
    let child = self::role("child");

    coll::add(&get(&role, "extendedBy"), Value::Object(child.clone())).unwrap();
    coll::add(&get(&child, "extends"), Value::Object(role.clone())).unwrap();

    em.persist(&role).unwrap();
    em.persist(&child).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let child = em
        .get_repository(&class("DDC992Role"))
        .unwrap()
        .find(get(&child, "roleID"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(1, count_of(&child, "extends").await);
    for parent in elements(&child, "extends").await {
        assert_loose_eq(&get(&role, "roleID"), &get(&parent, "roleID"));
    }
}

#[tokio::test]
async fn testOneToManyChild() {
    let em = set_up().await;

    let parent = new("DDC992Parent");
    let child = new("DDC992Child");
    set_obj(&child, "parent", &parent);
    append(&parent, "childs", Value::Object(child.clone()));

    em.persist(&parent).unwrap();
    em.persist(&child).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let parent_repository = em.get_repository(&class("DDC992Parent")).unwrap();
    let child_repository = em.get_repository(&class("DDC992Child")).unwrap();

    let parent = parent_repository
        .find(get(&parent, "id"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(1, count_of(&parent, "childs").await);
    let first_child = obj(&coll::get(&get(&parent, "childs"), 0).await.unwrap());
    assert_eq!(0, count_of(&first_child, "childs").await);

    let Value::Array(criteria) = array(&[("id", get(&child, "id"))]) else {
        unreachable!()
    };
    let child = parent_repository
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .unwrap();
    assert!(same(&first_child, &child));

    em.clear().unwrap();

    let child = parent_repository
        .find(get(&child, "id"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(0, count_of(&child, "childs").await);

    em.clear().unwrap();

    let child = child_repository
        .find(get(&child, "id"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(0, count_of(&child, "childs").await);
}
