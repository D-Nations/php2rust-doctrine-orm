//! Port of tests/Tests/ORM/Utility/IdentifierFlattenerEnumIdTest.php,
//! on `SQLite` with the recorded models.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Utility/IdentifierFlattenerEnumIdTest.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_persistence::php_reflection;

use orm::{get, new_object, set};

const TYPED_CARD_ENUM_ID: &str = "Doctrine\\Tests\\Models\\Enums\\TypedCardEnumId";
const TYPED_CARD_ENUM_COMPOSITE_ID: &str =
    "Doctrine\\Tests\\Models\\Enums\\TypedCardEnumCompositeId";
const SUIT: &str = "Doctrine\\Tests\\Models\\Enums\\Suit";
const UNIT: &str = "Doctrine\\Tests\\Models\\Enums\\Unit";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[TYPED_CARD_ENUM_ID, TYPED_CARD_ENUM_COMPOSITE_ID]).await
}

/// `$enum::Case`.
fn case(enum_type: &str, name: &str) -> Value {
    models::register();
    let class = php_reflection::class_info(enum_type).expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .expect("the case is declared");
    Value::Object(case)
}

fn same_case(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => Arc::ptr_eq(a, b),
        _ => false,
    }
}

#[tokio::test]
async fn testFlattenIdentifierWithEnumId() {
    let em = set_up().await;
    let typed_card_enum_id_entity = new_object(TYPED_CARD_ENUM_ID);
    set(&typed_card_enum_id_entity, "suit", case(SUIT, "Clubs"));

    em.persist(&typed_card_enum_id_entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let not_found = em
        .get_repository(TYPED_CARD_ENUM_ID)
        .unwrap()
        .find(case(SUIT, "Diamonds"))
        .await
        .unwrap();

    assert!(
        not_found.is_none(),
        "Search by non-persisted Enum ID does not work"
    );

    let found = em
        .get_repository(TYPED_CARD_ENUM_ID)
        .unwrap()
        .find(case(SUIT, "Clubs"))
        .await
        .unwrap();

    let found = found.expect("Search by Enum ID does not work");

    let class = em.get_class_metadata(TYPED_CARD_ENUM_ID).unwrap();

    let id = class.read().get_identifier_values(&found).unwrap();

    assert_eq!(id.len(), 1, "We should have 1 identifier");

    assert!(same_case(&case(SUIT, "Clubs"), &get(&found, "suit")));
}

#[tokio::test]
async fn testFlattenIdentifierWithCompositeEnumId() {
    let em = set_up().await;
    let entity = new_object(TYPED_CARD_ENUM_COMPOSITE_ID);
    set(&entity, "suit", case(SUIT, "Clubs"));
    set(&entity, "unit", case(UNIT, "Gram"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let id = |suit: &str| {
        let mut id = PhpArray::new();
        id.insert("suit", case(SUIT, suit));
        id.insert("unit", case(UNIT, "Gram"));
        Value::Array(id)
    };

    let not_found = em
        .get_repository(TYPED_CARD_ENUM_COMPOSITE_ID)
        .unwrap()
        .find(id("Diamonds"))
        .await
        .unwrap();

    assert!(
        not_found.is_none(),
        "Search by non-persisted composite Enum ID does not work"
    );

    let found = em
        .get_repository(TYPED_CARD_ENUM_COMPOSITE_ID)
        .unwrap()
        .find(id("Clubs"))
        .await
        .unwrap();

    let found = found.expect("Search by composite Enum ID does not work");

    let class = em.get_class_metadata(TYPED_CARD_ENUM_COMPOSITE_ID).unwrap();

    let identifier = class.read().get_identifier_values(&found).unwrap();

    assert_eq!(identifier.len(), 2, "We should have 2 identifiers");

    assert!(same_case(&case(SUIT, "Clubs"), &get(&found, "suit")));
    assert!(same_case(&case(UNIT, "Gram"), &get(&found, "unit")));
}
