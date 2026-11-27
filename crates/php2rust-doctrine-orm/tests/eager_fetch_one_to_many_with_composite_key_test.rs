//! Port of `EagerFetchOneToManyWithCompositeKeyTest`, on `SQLite`. The
//! model's constructor is written here, on its recorded class.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/EagerFetchOneToManyWithCompositeKeyTest.php

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
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::{new_object, s, set};

const NS: &str = "Doctrine\\Tests\\Models\\EagerFetchedCompositeOneToMany\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new RootEntity($id, $other)`.
fn root_entity(id: i64, other: &str) -> Object {
    let root = new_object(&class("RootEntity"));
    set(&root, "otherKey", s(other));
    set(
        &root,
        "secondLevel",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &root,
        "anotherSecondLevel",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&root, "id", Value::Int(id));
    root
}

#[tokio::test]
async fn testItDoesNotThrowAnExceptionWhenTriggeringALoad() {
    let em = orm::set_up(&[
        &class("RootEntity"),
        &class("SecondLevel"),
        &class("SecondLevelWithoutCompositePrimaryKey"),
    ])
    .await;

    let a1 = root_entity(1, "A");

    em.persist(&a1).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    assert_eq!(
        em.get_repository(&class("RootEntity"))
            .unwrap()
            .find_all()
            .await
            .unwrap()
            .len(),
        1
    );
}
