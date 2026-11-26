//! Ticket/DDC849Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC849Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn set_up() -> (Arc<EntityManager>, Object) {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("Benjamin", "beberlei", "active");

    let group1 = cms_group("Group 1");
    let group2 = cms_group("Group 2");

    add_group(&user, &group1);
    add_group(&user, &group2);

    em.persist(&user).unwrap();
    em.persist(&group1).unwrap();
    em.persist(&group2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    (em, user)
}

async fn group(user: &Object, key: i64) -> Value {
    coll::get(&get(user, "groups"), key).await.unwrap()
}

async fn contains(user: &Object, group: &Value) -> bool {
    coll::contains(&get(user, "groups"), group).await.unwrap()
}

#[tokio::test]
async fn testRemoveContains() {
    let (_em, user) = set_up().await;
    let group1 = group(&user, 0).await;
    let group2 = group(&user, 1).await;

    assert!(contains(&user, &group1).await);
    assert!(contains(&user, &group2).await);

    coll::remove_element(&get(&user, "groups"), &group1)
        .await
        .unwrap();
    coll::remove(&get(&user, "groups"), 1).await.unwrap();

    assert!(!contains(&user, &group1).await);
    assert!(!contains(&user, &group2).await);
}

#[tokio::test]
async fn testClearCount() {
    let (_em, user) = set_up().await;
    // new CmsGroup()
    let group = new_object(CMS_GROUP);
    set(
        &group,
        "users",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    add_group(&user, &group);

    assert_eq!(3, count_of(&user, "groups").await);

    coll::clear(&get(&user, "groups")).await.unwrap();

    assert_eq!(0, coll::count(&get(&user, "groups")).await.unwrap());
    assert_eq!(0, count_of(&user, "groups").await);
}

#[tokio::test]
async fn testClearContains() {
    let (_em, user) = set_up().await;
    let group1 = group(&user, 0).await;
    let group2 = group(&user, 1).await;

    assert!(contains(&user, &group1).await);
    assert!(contains(&user, &group2).await);

    coll::clear(&get(&user, "groups")).await.unwrap();

    assert!(!contains(&user, &group1).await);
    assert!(!contains(&user, &group2).await);
}
