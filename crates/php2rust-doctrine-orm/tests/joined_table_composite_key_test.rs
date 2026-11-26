//! JoinedTableCompositeKeyTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The `compositekeyinheritance` model set. `assertEquals($a, $b)` on
//! two entities compares their class and every property.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/JoinedTableCompositeKeyTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const NS: &str = "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\";
const MODELS: [&str; 4] = [
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedRootClass",
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedChildClass",
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\SingleRootClass",
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\SingleChildClass",
];

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&MODELS).await
}

/// `assertEquals($expected, $actual)` of two entities.
fn assert_entity_equals(expected: &Object, actual: &Object) {
    assert_eq!(expected.class(), actual.class());
    for property in ["keyPart1", "keyPart2", "extension", "additionalId"] {
        assert_loose_eq(&get(expected, property), &get(actual, property));
    }
}

async fn find_entity(em: &EntityManager) -> Object {
    let mut id = PhpArray::new();
    id.insert("keyPart1", s("part-1"));
    id.insert("keyPart2", s("part-2"));
    em.find(&format!("{NS}JoinedRootClass"), Value::Array(id))
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn testInsertWithCompositeKey() {
    let em = set_up().await;
    let child_entity = new_object(&format!("{NS}JoinedChildClass"));
    em.persist(&child_entity).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let entity = find_entity(&em).await;
    assert_entity_equals(&child_entity, &entity);
}

#[tokio::test]
async fn testUpdateWithCompositeKey() {
    let em = set_up().await;
    let child_entity = new_object(&format!("{NS}JoinedChildClass"));
    em.persist(&child_entity).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let entity = find_entity(&em).await;
    set(&entity, "extension", s("ext-new"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let persisted_entity = find_entity(&em).await;
    assert_entity_equals(&entity, &persisted_entity);
}
