//! Ticket/DDC6303Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC6303Test.php

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

use php_value::compare::loose_equals;
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

use orm::{get, new_object, s, set};

/// `[...]`.
fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

const DDC6303_BASE_CLASS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC6303BaseClass";
const DDC6303_CHILD_A: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC6303ChildA";
const DDC6303_CHILD_B: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC6303ChildB";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[DDC6303_BASE_CLASS, DDC6303_CHILD_A, DDC6303_CHILD_B]).await
}

/// `new DDC6303ChildA($id, $originalData)` / `new DDC6303ChildB(...)`.
fn child(class: &str, id: &str, original_data: Value) -> Object {
    let entity = new_object(class);
    set(&entity, "originalData", original_data);
    set(&entity, "id", s(id));
    entity
}

#[tokio::test]
async fn testMixedTypeHydratedCorrectlyInJoinedInheritance() {
    let em = set_up().await;
    // DDC6303ChildA and DDC6303ChildB have an inheritance from DDC6303BaseClass,
    // but one has a string originalData and the second has an array, since the fields
    // are mapped differently
    assert_hydrated_entities_same_to_persisted_ones(
        &em,
        vec![
            child(DDC6303_CHILD_A, "a", s("authorized")),
            child(
                DDC6303_CHILD_B,
                "b",
                list(vec![s("accepted"), s("authorized")]),
            ),
        ],
    )
    .await;
}

#[tokio::test]
async fn testEmptyValuesInJoinedInheritance() {
    let em = set_up().await;
    assert_hydrated_entities_same_to_persisted_ones(
        &em,
        vec![
            child(DDC6303_CHILD_A, "stringEmpty", s("")),
            child(DDC6303_CHILD_A, "stringZero", Value::Int(0)),
            child(DDC6303_CHILD_B, "arrayEmpty", Value::Array(PhpArray::new())),
        ],
    )
    .await;
}

/// `$persistedEntities` indexed by identifier.
async fn assert_hydrated_entities_same_to_persisted_ones(
    em: &Arc<EntityManager>,
    persisted_entities: Vec<Object>,
) {
    for entity in &persisted_entities {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();

    let ids: Vec<Value> = persisted_entities
        .iter()
        .map(|entity| get(entity, "id"))
        .collect();
    let mut qb = em
        .get_repository(DDC6303_BASE_CLASS)
        .unwrap()
        .create_query_builder("p", None)
        .unwrap();
    qb.r#where(vec![s("p.id IN(:ids)")])
        .unwrap()
        .set_parameter("ids", list(ids), None);
    let entities = qb
        .get_query()
        .unwrap()
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();
    let Value::Array(entities) = entities else {
        panic!("a list: {entities:?}");
    };

    assert_eq!(persisted_entities.len(), entities.len());

    for entity in entities.values() {
        let id = get(&orm::obj(entity), "id");
        let persisted = persisted_entities
            .iter()
            .find(|p| loose_equals(&get(p, "id"), &id))
            .expect("a persisted entity");
        assert!(
            loose_equals(entity, &Value::Object(persisted.clone())),
            "{entity:?} != {persisted:?}"
        );
    }
}
