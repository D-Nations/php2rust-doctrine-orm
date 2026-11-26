//! Ticket/GH11500Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11500Test.php

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

use php_value::compare::loose_equals;
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::{get, new_object, set};

const GH11500_ABSTRACT_TEST_ENTITY: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11500AbstractTestEntity";
const GH11500_TEST_ENTITY_ONE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11500TestEntityOne";
const GH11500_TEST_ENTITY_TWO: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11500TestEntityTwo";
const GH11500_TEST_ENTITY_HOLDER: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11500TestEntityHolder";

/// `new GH11500TestEntityHolder()`.
fn test_entity_holder() -> Object {
    let holder = new_object(GH11500_TEST_ENTITY_HOLDER);
    set(
        &holder,
        "testEntityOnes",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &holder,
        "testEntityTwos",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    holder
}

#[tokio::test]
async fn testDeleteOneToManyCollectionWithSingleTableInheritance() {
    let em = orm::set_up(&[
        GH11500_ABSTRACT_TEST_ENTITY,
        GH11500_TEST_ENTITY_ONE,
        GH11500_TEST_ENTITY_TWO,
        GH11500_TEST_ENTITY_HOLDER,
    ])
    .await;

    let test_entity_one = new_object(GH11500_TEST_ENTITY_ONE);
    let test_entity_two = new_object(GH11500_TEST_ENTITY_TWO);
    let test_entity_holder = test_entity_holder();

    set(
        &test_entity_one,
        "testEntityHolder",
        Value::Object(test_entity_holder.clone()),
    );
    coll::add(
        &get(&test_entity_holder, "testEntityOnes"),
        Value::Object(test_entity_one.clone()),
    )
    .unwrap();

    set(
        &test_entity_two,
        "testEntityHolder",
        Value::Object(test_entity_holder.clone()),
    );
    coll::add(
        &get(&test_entity_holder, "testEntityTwos"),
        Value::Object(test_entity_two.clone()),
    )
    .unwrap();

    em.persist(&test_entity_one).unwrap();
    em.persist(&test_entity_two).unwrap();
    em.persist(&test_entity_holder).unwrap();
    em.flush().await.unwrap();

    let test_entity_twos_before_removal_of_test_entity_ones =
        coll::to_array(&get(&test_entity_holder, "testEntityTwos"))
            .await
            .unwrap();

    set(
        &test_entity_holder,
        "testEntityOnes",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    em.persist(&test_entity_holder).unwrap();
    em.flush().await.unwrap();
    em.refresh(&test_entity_holder).await.unwrap();

    assert!(
        coll::to_array(&get(&test_entity_holder, "testEntityOnes"))
            .await
            .unwrap()
            .is_empty(),
        "All records should have been deleted"
    );
    assert!(
        loose_equals(
            &Value::Array(test_entity_twos_before_removal_of_test_entity_ones),
            &Value::Array(
                coll::to_array(&get(&test_entity_holder, "testEntityTwos"))
                    .await
                    .unwrap()
            )
        ),
        "Different Entity's records should not have been deleted"
    );
}
