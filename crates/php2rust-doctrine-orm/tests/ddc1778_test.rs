//! Ticket/DDC1778Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1778Test.php

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
    user: Object,
    phone: Object,
}

async fn set_up() -> T {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user_with("Benjamin", "beberlei", "active");

    let phone = cms_phonenumber("0123456789");
    add_phonenumber(&user, &phone);

    em.persist(&user).unwrap();
    em.persist(&phone).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    let phone = em
        .find(CMS_PHONENUMBER, get(&phone, "phonenumber"))
        .await
        .unwrap()
        .unwrap();
    T { em, user, phone }
}

impl T {
    /// `clone $this->user->getPhonenumbers()`.
    async fn cloned_numbers(&self) -> Value {
        PersistentCollection::from_value(&get(&self.user, "phonenumbers"))
            .unwrap()
            .clone_collection()
            .await
            .unwrap()
            .value()
    }

    /// Flushes, clears, and counts the phonenumbers of the reloaded user.
    async fn phonenumbers_after_flush(&self) -> usize {
        self.em.flush().await.unwrap();
        self.em.clear().unwrap();

        let user = self
            .em
            .find(CMS_USER, get(&self.user, "id"))
            .await
            .unwrap()
            .unwrap();
        count_of(&user, "phonenumbers").await
    }
}

#[tokio::test]
async fn testClear() {
    let t = set_up().await;
    let cloned_numbers = t.cloned_numbers().await;
    coll::clear(&cloned_numbers).await.unwrap();

    assert_eq!(1, t.phonenumbers_after_flush().await);
}

#[tokio::test]
async fn testRemove() {
    let t = set_up().await;
    let cloned_numbers = t.cloned_numbers().await;
    coll::remove(&cloned_numbers, 0).await.unwrap();

    assert_eq!(1, t.phonenumbers_after_flush().await);
}

#[tokio::test]
async fn testRemoveElement() {
    let t = set_up().await;
    let cloned_numbers = t.cloned_numbers().await;
    coll::remove_element(&cloned_numbers, &Value::Object(t.phone.clone()))
        .await
        .unwrap();

    assert_eq!(1, t.phonenumbers_after_flush().await);
}
