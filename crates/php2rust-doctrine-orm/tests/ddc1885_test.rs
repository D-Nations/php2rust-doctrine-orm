//! Ticket/DDC1885Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the quote models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1885Test.php

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
#[path = "common/quote.rs"]
mod quote;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use quote::*;

async fn set_up() -> (Arc<EntityManager>, Object) {
    let em = orm::set_up(&QUOTE_MODELS).await;

    let user = quote_user("FabioBatSilva");
    append(&user, "groups", Value::Object(quote_group("G 1", None)));
    append(&user, "groups", Value::Object(quote_group("G 2", None)));

    // Create
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    (em, user)
}

async fn group(user: &Object, key: i64) -> Value {
    coll::get(&get(user, "groups"), key).await.unwrap()
}

/// `assertInstanceOf(User::class, $user)` and its name and id.
fn assert_user(user: &Object, id: &Value) {
    assert_eq!(QUOTE_USER, user.class());
    assert_loose_eq(&s("FabioBatSilva"), &get(user, "name"));
    assert_loose_eq(id, &get(user, "id"));
}

async fn assert_two_groups(user: &Object) {
    assert_eq!(2, count_of(user, "groups").await);
    assert_eq!(QUOTE_GROUP, obj(&group(user, 0).await).class());
    assert_eq!(QUOTE_GROUP, obj(&group(user, 1).await).class());
}

#[tokio::test]
async fn testCreateRetrieveUpdateDelete() {
    let (em, user) = set_up().await;
    let g1 = obj(&group(&user, 0).await);
    let g2 = obj(&group(&user, 1).await);

    let u1_id = get(&user, "id");
    let g1_id = get(&g1, "id");
    let g2_id = get(&g2, "id");

    // Retrieve
    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);

    assert_eq!(2, count_of(&user, "groups").await);

    let g1 = obj(&group(&user, 0).await);
    let g2 = obj(&group(&user, 1).await);

    assert_eq!(QUOTE_GROUP, g1.class());
    assert_eq!(QUOTE_GROUP, g2.class());

    set(&g1, "name", s("Bar 11"));
    set(&g2, "name", s("Foo 22"));

    // Update
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);

    // Delete
    em.remove(&user).await.unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    assert!(em.find(QUOTE_USER, u1_id).await.unwrap().is_none());
    assert!(em.find(QUOTE_GROUP, g1_id).await.unwrap().is_none());
    assert!(em.find(QUOTE_GROUP, g2_id).await.unwrap().is_none());
}

#[tokio::test]
async fn testRemoveItem() {
    let (em, user) = set_up().await;
    let u1_id = get(&user, "id");
    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);
    assert_two_groups(&user).await;

    coll::remove(&get(&user, "groups"), 0).await.unwrap();

    // Update
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);

    assert_eq!(1, count_of(&user, "groups").await);
}

#[tokio::test]
async fn testClearAll() {
    let (em, user) = set_up().await;
    let u1_id = get(&user, "id");
    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);
    assert_two_groups(&user).await;

    coll::clear(&get(&user, "groups")).await.unwrap();

    // Update
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);

    assert_eq!(0, count_of(&user, "groups").await);
}

#[tokio::test]
async fn testCountExtraLazy() {
    let (em, user) = set_up().await;
    let u1_id = get(&user, "id");
    let user = em.find(QUOTE_USER, u1_id.clone()).await.unwrap().unwrap();

    assert_user(&user, &u1_id);
    assert_two_groups(&user).await;
}
