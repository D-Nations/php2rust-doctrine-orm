//! Port of `InsertableUpdatableTest`, on `SQLite` with the `Upsertable`
//! models.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/InsertableUpdatableTest.php

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

use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{assert_loose_eq, get, new_object, s, set};

const INSERTABLE: &str = "Doctrine\\Tests\\Models\\Upsertable\\Insertable";
const UPDATABLE: &str = "Doctrine\\Tests\\Models\\Upsertable\\Updatable";

/// `setUp()`: `createSchemaForModels(Updatable::class, Insertable::class)`.
async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[UPDATABLE, INSERTABLE]).await
}

#[tokio::test]
async fn testNotInsertableIsFetchedFromDatabase() {
    let em = set_up().await;
    let insertable = new_object(INSERTABLE);
    set(&insertable, "insertableContent", s("abcdefg"));

    em.persist(&insertable).unwrap();
    em.flush().await.unwrap();

    // gets inserted from default value and fetches value from database
    assert_loose_eq(&s("1234"), &get(&insertable, "nonInsertableContent"));

    set(&insertable, "nonInsertableContent", s("5678"));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let insertable = em
        .find(INSERTABLE, get(&insertable, "id"))
        .await
        .unwrap()
        .unwrap();

    // during UPDATE statement it is not ignored
    assert_loose_eq(&s("5678"), &get(&insertable, "nonInsertableContent"));
}

#[tokio::test]
async fn testNotUpdatableIsFetched() {
    let em = set_up().await;
    let updatable = new_object(UPDATABLE);
    set(&updatable, "updatableContent", s("foo"));
    set(&updatable, "nonUpdatableContent", s("foo"));

    em.persist(&updatable).unwrap();
    em.flush().await.unwrap();

    set(&updatable, "updatableContent", s("bar"));
    set(&updatable, "nonUpdatableContent", s("baz"));

    em.flush().await.unwrap();

    assert_loose_eq(&s("foo"), &get(&updatable, "nonUpdatableContent"));

    em.clear().unwrap();

    let clean_updatable = em
        .find(UPDATABLE, get(&updatable, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_loose_eq(&s("bar"), &get(&clean_updatable, "updatableContent"));
    assert_loose_eq(&s("foo"), &get(&clean_updatable, "nonUpdatableContent"));
}
