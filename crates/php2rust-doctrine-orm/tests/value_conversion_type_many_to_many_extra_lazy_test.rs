//! ManyToManyExtraLazyTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The entities all use a custom type that converts the value as
//! identifier(s) (`common/rot13.rs`). Each test has its own database, so
//! `tearDownAfterClass()` has nothing to drop.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/ManyToManyExtraLazyTest.php

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
#[path = "common/rot13.rs"]
mod rot13;

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const INVERSED: &str =
    "Doctrine\\Tests\\Models\\ValueConversionType\\InversedManyToManyExtraLazyEntity";
const OWNING: &str =
    "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToManyExtraLazyEntity";

fn entity(class: &str, id_field: &str, id: &str) -> Object {
    let entity = new_object(class);
    set(
        &entity,
        "associatedEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&entity, id_field, s(id));
    entity
}

/// `$a->associatedEntities->add($b)`.
fn add(a: &Object, b: &Object) {
    coll::add(&get(a, "associatedEntities"), Value::Object(b.clone())).unwrap();
}

async fn set_up() -> Arc<EntityManager> {
    rot13::register_rot13_type();
    let em = orm::set_up(&[INVERSED, OWNING]).await;

    let inversed1 = entity(INVERSED, "id1", "abc");
    let inversed2 = entity(INVERSED, "id1", "def");
    let owning1 = entity(OWNING, "id2", "ghi");
    let owning2 = entity(OWNING, "id2", "jkl");

    add(&inversed1, &owning1);
    add(&owning1, &inversed1);
    add(&inversed1, &owning2);
    add(&owning2, &inversed1);

    add(&inversed2, &owning1);
    add(&owning1, &inversed2);
    add(&inversed2, &owning2);
    add(&owning2, &inversed2);

    em.persist(&inversed1).unwrap();
    em.persist(&inversed2).unwrap();
    em.persist(&owning1).unwrap();
    em.persist(&owning2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

async fn find(em: &EntityManager, class: &str, id: &str) -> Object {
    em.find(class, s(id)).await.unwrap().unwrap()
}

#[tokio::test]
async fn testThatTheExtraLazyCollectionFromOwningToInversedIsCounted() {
    let em = set_up().await;
    let owning = find(&em, OWNING, "ghi").await;

    assert_eq!(
        coll::count(&get(&owning, "associatedEntities"))
            .await
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn testThatTheExtraLazyCollectionFromInversedToOwningIsCounted() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    assert_eq!(
        coll::count(&get(&inversed, "associatedEntities"))
            .await
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn testThatTheExtraLazyCollectionFromOwningToInversedContainsAnEntity() {
    let em = set_up().await;
    let owning = find(&em, OWNING, "ghi").await;

    let inversed = find(&em, INVERSED, "abc").await;

    assert!(
        coll::contains(
            &get(&owning, "associatedEntities"),
            &Value::Object(inversed)
        )
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn testThatTheExtraLazyCollectionFromInversedToOwningContainsAnEntity() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    let owning = find(&em, OWNING, "ghi").await;

    assert!(
        coll::contains(
            &get(&inversed, "associatedEntities"),
            &Value::Object(owning)
        )
        .await
        .unwrap()
    );
}

#[tokio::test]
async fn testThatTheExtraLazyCollectionFromOwningToInversedContainsAnIndexByKey() {
    let em = set_up().await;
    let owning = find(&em, OWNING, "ghi").await;

    assert!(
        coll::contains_key(&get(&owning, "associatedEntities"), "abc")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testThatTheExtraLazyCollectionFromInversedToOwningContainsAnIndexByKey() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    assert!(
        coll::contains_key(&get(&inversed, "associatedEntities"), "ghi")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testThatASliceOfTheExtraLazyCollectionFromOwningToInversedIsLoaded() {
    let em = set_up().await;
    let owning = find(&em, OWNING, "ghi").await;

    assert_eq!(
        coll::slice(&get(&owning, "associatedEntities"), 0, Some(1))
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn testThatASliceOfTheExtraLazyCollectionFromInversedToOwningIsLoaded() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    assert_eq!(
        coll::slice(&get(&inversed, "associatedEntities"), 1, Some(1))
            .await
            .unwrap()
            .len(),
        1
    );
}
