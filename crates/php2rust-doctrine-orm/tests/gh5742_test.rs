//! Ticket/GH5742Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with the foreign keys enforced as
//! upstream's `EnableForeignKeys` middleware does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5742Test.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    set_up_with_foreign_keys(&["GH5742Person", "GH5742Toothbrush", "GH5742ToothpasteBrand"]).await
}

/// `new GH5742Person()`.
fn person() -> Object {
    let person = new("GH5742Person");
    set(
        &person,
        "preferredBrands",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    person
}

#[tokio::test]
async fn testUpdateOneToOneToNewEntityBeforePreviousEntityCanBeRemoved() {
    let em = set_up().await;

    let person = person();
    let old_toothbrush = new("GH5742Toothbrush");
    set_obj(&person, "toothbrush", &old_toothbrush);

    em.persist(&person).unwrap();
    em.persist(&old_toothbrush).unwrap();
    em.flush().await.unwrap();

    let old_toothbrush_id = get(&old_toothbrush, "id");

    let new_toothbrush = new("GH5742Toothbrush");
    set_obj(&person, "toothbrush", &new_toothbrush);

    em.remove(&old_toothbrush).await.unwrap();
    em.persist(&new_toothbrush).unwrap();

    // The flush operation will have to make sure the new toothbrush
    // has been written to the database
    // _before_ the person can be updated to refer to it.
    // Likewise, the update must have happened _before_ the old
    // toothbrush can be removed (non-nullable FK constraint).

    em.flush().await.unwrap();

    em.clear().unwrap();
    let found = find(&em, "GH5742Person", get(&person, "id")).await.unwrap();
    assert_eq!(
        get(&new_toothbrush, "id"),
        get(&obj(&get(&found, "toothbrush")), "id")
    );
    assert!(
        find(&em, "GH5742Toothbrush", old_toothbrush_id)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn testManyToManyCollectionUpdateBeforeRemoval() {
    let em = set_up().await;

    let person = person();
    set_obj(&person, "toothbrush", &new("GH5742Toothbrush")); // to satisfy not-null constraint
    em.persist(&person).unwrap();

    let old_mice = new("GH5742ToothpasteBrand");
    em.persist(&old_mice).unwrap();

    coll::set(
        &get(&person, "preferredBrands"),
        1,
        Value::Object(old_mice.clone()),
    )
    .await
    .unwrap();
    em.flush().await.unwrap();

    let old_brand_id = get(&old_mice, "id");

    let new_spice = new("GH5742ToothpasteBrand");
    em.persist(&new_spice).unwrap();

    coll::set(
        &get(&person, "preferredBrands"),
        1,
        Value::Object(new_spice.clone()),
    )
    .await
    .unwrap();

    em.remove(&old_mice).await.unwrap();

    // The flush operation will have to make sure the new brand
    // has been written to the database _before_ it can be referred
    // to from the m2m join table.
    // Likewise, the old join table entry must have been removed
    // _before_ the old brand can be removed.

    em.flush().await.unwrap();

    em.clear().unwrap();
    let found = find(&em, "GH5742Person", get(&person, "id")).await.unwrap();
    assert_eq!(1, count_of(&found, "preferredBrands").await);
    assert!(
        find(&em, "GH5742ToothpasteBrand", old_brand_id)
            .await
            .is_none()
    );
}
