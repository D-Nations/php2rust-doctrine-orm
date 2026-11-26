//! Ticket/DDC2084Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `initialize_object()` where PHP calls `getValue()` on a proxy.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2084Test.php

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

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const MY_ENTITY1: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2084\\MyEntity1";
const MY_ENTITY2: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2084\\MyEntity2";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[MY_ENTITY1, MY_ENTITY2]).await
}

/// `new MyEntity2($value)`.
fn my_entity2(value: &str) -> Object {
    let e2 = new_object(MY_ENTITY2);
    set(&e2, "value", s(value));
    e2
}

async fn load_fixture(em: &EntityManager) -> Object {
    let e2 = my_entity2("Foo");
    // new MyEntity1($e2)
    let e1 = new_object(MY_ENTITY1);
    set_obj(&e1, "entity2", &e2);

    em.persist(&e2).unwrap();
    em.flush().await.unwrap();

    em.persist(&e1).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    e1
}

#[tokio::test]
async fn testIssue() {
    let em = set_up().await;
    let e1 = load_fixture(&em).await;
    let e2 = get(&e1, "entity2");
    let e = em.find(MY_ENTITY1, e2).await.unwrap().unwrap();
    assert!(e.instance_of(MY_ENTITY1));
    let my_entity2 = get(&e, "entity2");
    assert!(matches!(&my_entity2, Value::Object(o) if o.instance_of(MY_ENTITY2)));
    let my_entity2 = obj(&my_entity2);
    em.initialize_object(&my_entity2).await.unwrap();
    assert_eq!(s("Foo"), get(&my_entity2, "value"));
}

#[tokio::test]
async fn testInvalidIdentifierBindingEntityException() {
    let em = set_up().await;
    let error = em
        .find(MY_ENTITY1, Value::Object(my_entity2("Foo")))
        .await
        .expect_err("an ORMInvalidArgumentException");
    assert_eq!("Doctrine\\ORM\\ORMInvalidArgumentException", error.class);
    assert_eq!(
        "Binding entities to query parameters only allowed for entities that have an identifier.\nClass \"Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2084\\MyEntity2\" does not have an identifier.",
        message(&error)
    );
}
