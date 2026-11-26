//! Ticket/GH7717Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/GH7717 models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7717Test.php

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
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;

const GH7717_PARENT: &str = "Doctrine\\Tests\\Models\\GH7717\\GH7717Parent";
const GH7717_CHILD: &str = "Doctrine\\Tests\\Models\\GH7717\\GH7717Child";

#[tokio::test]
async fn testManyToManyPersisterIsNullComparison() {
    let em = orm::set_up(&[GH7717_PARENT, GH7717_CHILD]).await;
    let child_with_null_property = new_object(GH7717_CHILD);
    let child_without_null_property = new_object(GH7717_CHILD);
    set(&child_without_null_property, "nullableProperty", s("nope"));

    let parent = new_object(GH7717_PARENT);
    set(
        &parent,
        "children",
        ArrayCollectionObject::value(PhpArray::list([
            Value::Object(child_with_null_property),
            Value::Object(child_without_null_property),
        ])),
    );

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let parent = em
        .find(GH7717_PARENT, Value::Int(1))
        .await
        .unwrap()
        .unwrap();

    // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().is_null("nullableProperty"));
    let matched = coll::matching(&get(&parent, "children"), &criteria)
        .await
        .unwrap();
    assert_eq!(1, coll::count(&matched).await.unwrap());
}
