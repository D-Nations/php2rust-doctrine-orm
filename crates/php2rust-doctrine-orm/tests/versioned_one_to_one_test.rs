//! Port of `VersionedOneToOneTest`, on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/VersionedOneToOneTest.php

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

use php_value::{PhpArray, Value};

use orm::{get_object, new_object, s, same, set};

const FIRST: &str = "Doctrine\\Tests\\Models\\VersionedOneToOne\\FirstRelatedEntity";
const SECOND: &str = "Doctrine\\Tests\\Models\\VersionedOneToOne\\SecondRelatedEntity";

fn by_name(name: &str) -> PhpArray {
    let mut criteria = PhpArray::new();
    criteria.insert("name", s(name));
    criteria
}

/// This test case tests that a versionable entity, that has a oneToOne relationship as it's id can be created
///  without this bug fix (DDC-3318), you could not do this
#[tokio::test]
async fn testSetVersionOnCreate() {
    let em = orm::set_up(&[FIRST, SECOND]).await;

    let second_related_entity = new_object(SECOND);
    set(&second_related_entity, "name", s("Bob"));

    em.persist(&second_related_entity).unwrap();
    em.flush().await.unwrap();

    let first_related_entity = new_object(FIRST);
    set(&first_related_entity, "name", s("Fred"));
    set(
        &first_related_entity,
        "secondEntity",
        Value::Object(second_related_entity.clone()),
    );

    em.persist(&first_related_entity).unwrap();
    em.flush().await.unwrap();

    let first_entity = em
        .get_repository(FIRST)
        .unwrap()
        .find_one_by(&by_name("Fred"), None)
        .await
        .unwrap()
        .unwrap();

    let second_entity = em
        .get_repository(SECOND)
        .unwrap()
        .find_one_by(&by_name("Bob"), None)
        .await
        .unwrap()
        .unwrap();

    assert!(same(&first_related_entity, &first_entity));
    assert!(same(&second_related_entity, &second_entity));
    assert!(same(
        &get_object(&first_entity, "secondEntity").unwrap(),
        &second_entity
    ));
}
