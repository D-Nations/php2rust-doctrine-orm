//! Ticket/DDC2106Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2106Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `new DDC2106Entity()`.
fn entity() -> Object {
    let entity = new("DDC2106Entity");
    set(
        &entity,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    entity
}

#[tokio::test]
async fn testDetachedEntityAsId() {
    let em = ticket::set_up(&["DDC2106Entity"]).await;

    let e = entity();
    em.persist(&e).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let e = em
        .get_repository(&class("DDC2106Entity"))
        .unwrap()
        .find_one_by(&PhpArray::new(), None)
        .await
        .unwrap()
        .unwrap();

    let entity_without_id = entity();
    em.persist(&entity_without_id).unwrap();

    // `Criteria::ASC` is not defined in collections 3: `Criteria::create()`.
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("parent", Value::Object(entity_without_id)));

    let matched = coll::matching(&get(&e, "children"), &criteria)
        .await
        .unwrap();
    assert_eq!(0, coll::count(&matched).await.unwrap());
}
