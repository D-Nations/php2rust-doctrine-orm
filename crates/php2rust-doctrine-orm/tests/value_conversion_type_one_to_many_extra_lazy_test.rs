//! OneToManyExtraLazyTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The entities all use a custom type that converts the value as
//! identifier(s) (`common/rot13.rs`). Each test has its own database, so
//! `tearDownAfterClass()` has nothing to drop.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ValueConversionType/OneToManyExtraLazyTest.php

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
    "Doctrine\\Tests\\Models\\ValueConversionType\\InversedOneToManyExtraLazyEntity";
const OWNING: &str = "Doctrine\\Tests\\Models\\ValueConversionType\\OwningManyToOneExtraLazyEntity";

async fn set_up() -> Arc<EntityManager> {
    rot13::register_rot13_type();
    let em = orm::set_up(&[INVERSED, OWNING]).await;

    let inversed = new_object(INVERSED);
    set(
        &inversed,
        "associatedEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&inversed, "id1", s("abc"));

    let mut owning = Vec::new();
    for id2 in ["def", "ghi", "jkl"] {
        let entity = new_object(OWNING);
        set(&entity, "id2", s(id2));
        coll::add(
            &get(&inversed, "associatedEntities"),
            Value::Object(entity.clone()),
        )
        .unwrap();
        set_obj(&entity, "associatedEntity", &inversed);
        owning.push(entity);
    }

    em.persist(&inversed).unwrap();
    for entity in &owning {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

async fn find(em: &EntityManager, class: &str, id: &str) -> Object {
    em.find(class, s(id)).await.unwrap().unwrap()
}

#[tokio::test]
async fn testThatExtraLazyCollectionIsCounted() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    assert_eq!(
        coll::count(&get(&inversed, "associatedEntities"))
            .await
            .unwrap(),
        3
    );
}

#[tokio::test]
async fn testThatExtraLazyCollectionContainsAnEntity() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    let owning = find(&em, OWNING, "def").await;

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
async fn testThatExtraLazyCollectionContainsAnIndexbyKey() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    assert!(
        coll::contains_key(&get(&inversed, "associatedEntities"), "def")
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn testThatASliceOfTheExtraLazyCollectionIsLoaded() {
    let em = set_up().await;
    let inversed = find(&em, INVERSED, "abc").await;

    assert_eq!(
        coll::slice(&get(&inversed, "associatedEntities"), 0, Some(2))
            .await
            .unwrap()
            .len(),
        2
    );
}
