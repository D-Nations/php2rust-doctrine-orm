//! Ticket/DDC1643Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1643Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;

struct T {
    em: Arc<EntityManager>,
    user1: Object,
    user2: Object,
}

async fn set_up() -> T {
    let em = orm::set_up(&CMS_MODELS).await;

    let user1 = cms_user_with("Benjamin", "beberlei", "active");

    let group1 = cms_group("test");
    let group2 = cms_group("test");

    add_group(&user1, &group1);
    add_group(&user1, &group2);

    let user2 = cms_user_with("Roman", "romanb", "active");

    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.persist(&group1).unwrap();
    em.persist(&group2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user1 = em.find(CMS_USER, get(&user1, "id")).await.unwrap().unwrap();
    let user2 = em.find(CMS_USER, get(&user2, "id")).await.unwrap().unwrap();
    T { em, user1, user2 }
}

/// `clone $user->groups`.
async fn clone_groups(user: &Object) -> Value {
    PersistentCollection::from_value(&get(user, "groups"))
        .unwrap()
        .clone_collection()
        .await
        .unwrap()
        .value()
}

impl T {
    async fn find_user(&self, user: &Object) -> Object {
        self.em
            .find(CMS_USER, get(user, "id"))
            .await
            .unwrap()
            .unwrap()
    }

    async fn flush_clear(&self) {
        self.em.flush().await.unwrap();
        self.em.clear().unwrap();
    }
}

#[tokio::test]
async fn testClonePersistentCollectionAndReuse() {
    let t = set_up().await;
    let user1 = t.user1.clone();

    set(&user1, "groups", clone_groups(&user1).await);

    t.flush_clear().await;

    let user1 = t.find_user(&user1).await;

    assert_eq!(2, count_of(&user1, "groups").await);
}

#[tokio::test]
async fn testClonePersistentCollectionAndShare() {
    let t = set_up().await;
    let user1 = t.user1.clone();
    let user2 = t.user2.clone();

    set(&user2, "groups", clone_groups(&user1).await);

    t.flush_clear().await;

    let user1 = t.find_user(&user1).await;
    let user2 = t.find_user(&user2).await;

    assert_eq!(2, count_of(&user1, "groups").await);
    assert_eq!(2, count_of(&user2, "groups").await);
}

#[tokio::test]
async fn testCloneThenDirtyPersistentCollection() {
    let t = set_up().await;
    let user1 = t.user1.clone();
    let user2 = t.user2.clone();

    let group3 = cms_group("test");
    set(&user2, "groups", clone_groups(&user1).await);
    coll::add(&get(&user2, "groups"), Value::Object(group3.clone())).unwrap();

    t.em.persist(&group3).unwrap();
    t.flush_clear().await;

    let user1 = t.find_user(&user1).await;
    let user2 = t.find_user(&user2).await;

    assert_eq!(3, count_of(&user2, "groups").await);
    assert_eq!(2, count_of(&user1, "groups").await);
}

#[tokio::test]
async fn testNotCloneAndPassAroundFlush() {
    let t = set_up().await;
    let user1 = t.user1.clone();
    let user2 = t.user2.clone();

    let group3 = cms_group("test");
    set(&user2, "groups", get(&user1, "groups"));
    coll::add(&get(&user2, "groups"), Value::Object(group3.clone())).unwrap();

    assert_eq!(
        1,
        PersistentCollection::from_value(&get(&user1, "groups"))
            .unwrap()
            .get_insert_diff()
            .len()
    );

    t.em.persist(&group3).unwrap();
    t.flush_clear().await;

    let user1 = t.find_user(&user1).await;
    let user2 = t.find_user(&user2).await;

    assert_eq!(3, count_of(&user2, "groups").await);
    assert_eq!(3, count_of(&user1, "groups").await);
}
