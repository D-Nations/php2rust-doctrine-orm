//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToManyOrphanRemovalTest.php

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

use php_value::Value;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

/// `setUp()`: a user with two phone numbers; the user's id.
async fn set_up_test() -> (Arc<EntityManager>, Value) {
    let em = set_up(&CMS_MODELS).await;
    let user = cms_user_with("Roman B.", "romanb", "dev");

    let phone1 = cms_phonenumber("123456");
    let phone2 = cms_phonenumber("234567");

    add_phonenumber(&user, &phone1);
    add_phonenumber(&user, &phone2);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");
    em.clear().unwrap();
    (em, user_id)
}

async fn count_of_query(em: &Arc<EntityManager>, dql: &str) -> usize {
    let result = em.create_query(dql).get_object_result().await.unwrap();
    list(&result).len()
}

const PHONENUMBERS: &str = "SELECT p FROM Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber p";

#[tokio::test]
async fn testOrphanRemoval() {
    let (em, user_id) = set_up_test().await;
    let user_proxy = em.get_reference(CMS_USER, user_id).await.unwrap().unwrap();

    em.remove(&user_proxy).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_eq!(
        0,
        count_of_query(&em, "SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u").await,
        "CmsUser should be removed by EntityManager"
    );

    assert_eq!(
        0,
        count_of_query(&em, PHONENUMBERS).await,
        "CmsPhonenumber should be removed by orphanRemoval"
    );
}

#[tokio::test]
async fn testOrphanRemovalRemoveFromCollection() {
    let (em, user_id) = set_up_test().await;
    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    coll::remove(&get(&user, "phonenumbers"), 0).await.unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_eq!(
        1,
        count_of_query(&em, PHONENUMBERS).await,
        "CmsPhonenumber should be removed by orphanRemoval"
    );
}

#[tokio::test]
async fn testOrphanRemovalClearCollectionAndReAdd() {
    let (em, user_id) = set_up_test().await;
    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    let phone1 = obj(&coll::first(&get(&user, "phonenumbers")).await.unwrap());

    coll::clear(&get(&user, "phonenumbers")).await.unwrap();
    add_phonenumber(&user, &phone1);

    em.flush().await.unwrap();

    assert_eq!(
        1,
        count_of_query(&em, PHONENUMBERS).await,
        "CmsPhonenumber should be removed by orphanRemoval"
    );
}

#[tokio::test]
async fn testOrphanRemovalClearCollectionAndAddNew() {
    let (em, user_id) = set_up_test().await;
    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();
    let new_phone = cms_phonenumber("654321");

    coll::clear(&get(&user, "phonenumbers")).await.unwrap();
    add_phonenumber(&user, &new_phone);

    em.flush().await.unwrap();

    assert_eq!(
        1,
        count_of_query(&em, PHONENUMBERS).await,
        "Old CmsPhonenumbers should be removed by orphanRemoval and new one added"
    );
}

#[tokio::test]
async fn testOrphanRemovalUnitializedCollection() {
    let (em, user_id) = set_up_test().await;
    let user = em.find(CMS_USER, user_id).await.unwrap().unwrap();

    coll::clear(&get(&user, "phonenumbers")).await.unwrap();
    em.flush().await.unwrap();

    assert_eq!(
        0,
        count_of_query(&em, PHONENUMBERS).await,
        "CmsPhonenumber should be removed by orphanRemoval"
    );
}
