//! Ticket/GH11501Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11501Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::{get, new_object, set};

const GH11501_ABSTRACT_TEST_ENTITY: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11501AbstractTestEntity";
const GH11501_TEST_ENTITY_ONE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11501TestEntityOne";
const GH11501_TEST_ENTITY_TWO: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11501TestEntityTwo";
const GH11501_TEST_ENTITY_HOLDER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11501TestEntityHolder";

/// `new GH11501TestEntityHolder()`.
fn test_entity_holder() -> Object {
    let holder = new_object(GH11501_TEST_ENTITY_HOLDER);
    set(
        &holder,
        "testEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    holder
}

#[tokio::test]
async fn testDeleteOneToManyCollectionWithSingleTableInheritance() {
    let em = orm::set_up(&[
        GH11501_ABSTRACT_TEST_ENTITY,
        GH11501_TEST_ENTITY_ONE,
        GH11501_TEST_ENTITY_TWO,
        GH11501_TEST_ENTITY_HOLDER,
    ])
    .await;

    let test_entity_one = new_object(GH11501_TEST_ENTITY_ONE);
    let test_entity_two = new_object(GH11501_TEST_ENTITY_TWO);
    let test_entity_holder = test_entity_holder();

    set(
        &test_entity_one,
        "testEntityHolder",
        Value::Object(test_entity_holder.clone()),
    );
    coll::add(
        &get(&test_entity_holder, "testEntities"),
        Value::Object(test_entity_one.clone()),
    )
    .unwrap();

    set(
        &test_entity_two,
        "testEntityHolder",
        Value::Object(test_entity_holder.clone()),
    );
    coll::add(
        &get(&test_entity_holder, "testEntities"),
        Value::Object(test_entity_two.clone()),
    )
    .unwrap();

    em.persist(&test_entity_one).unwrap();
    em.persist(&test_entity_two).unwrap();
    em.persist(&test_entity_holder).unwrap();
    em.flush().await.unwrap();

    set(
        &test_entity_holder,
        "testEntities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    em.persist(&test_entity_holder).unwrap();
    em.flush().await.unwrap();
    em.refresh(&test_entity_holder).await.unwrap();

    assert!(
        coll::to_array(&get(&test_entity_holder, "testEntities"))
            .await
            .unwrap()
            .is_empty(),
        "All records should have been deleted"
    );
}
