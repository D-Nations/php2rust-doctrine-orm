//! Functional/IdentityMapTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/IdentityMapTest.php

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

use php_value::{Object, PhpArray, Value, php_array};
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::context::{HINT_REFRESH, Hint};

use orm::{
    CMS_ADDRESS, CMS_MODELS, CMS_USER, add_phonenumber, cms_address, cms_phonenumber,
    cms_user_with, count_of, get, same, same_value, set, set_address,
};

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&CMS_MODELS).await
}

async fn execute_statement(em: &EntityManager, sql: &str, params: PhpArray) {
    em.get_connection()
        .execute_statement(sql, &params, &ParameterTypes::new())
        .await
        .unwrap();
}

async fn get_single_result(em: &Arc<EntityManager>, dql: &str, refresh: bool) -> Object {
    let q = em.create_query(dql);
    if refresh {
        q.set_hint(HINT_REFRESH, Hint::Value(Value::Bool(true)));
    }
    match q.get_single_result(None).await.unwrap() {
        Value::Object(object) => object,
        other => panic!("not an object: {other:?}"),
    }
}

/// `$address->setUser($user)`.
fn set_user(address: &Object, user: &Object) {
    if !same_value(&get(address, "user"), user) {
        set(address, "user", Value::Object(user.clone()));
        set_address(user, address);
    }
}

fn address_with_user1_user2() -> (Object, Object, Object) {
    let address = cms_address("de", "12345", "Berlin");

    let user1 = cms_user_with("Roman B.", "romanb", "dev");

    let user2 = cms_user_with("Guilherme Blanco", "gblanco", "dev");

    set_user(&address, &user1);

    (address, user1, user2)
}

#[tokio::test]
async fn testBasicIdentityManagement() {
    let em = set_up().await;
    let user = cms_user_with("Roman B.", "romanb", "dev");

    let address = cms_address("de", "1234", "Berlin");
    set(&address, "zip", Value::Int(1234));

    set_address(&user, &address);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user2 = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert!(!same(&user2, &user));
    let user3 = em.find(CMS_USER, get(&user, "id")).await.unwrap().unwrap();
    assert!(same(&user2, &user3));

    let address2 = em
        .find(CMS_ADDRESS, get(&address, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(!same(&address2, &address));
    let address3 = em
        .find(CMS_ADDRESS, get(&address, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(same(&address2, &address3));

    assert!(same_value(&get(&user2, "address"), &address2)); // !!!
}

#[tokio::test]
async fn testSingleValuedAssociationIdentityMapBehaviorWithRefresh() {
    let em = set_up().await;
    let (address, user1, user2) = address_with_user1_user2();

    em.persist(&address).unwrap();
    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();

    assert!(same_value(&get(&address, "user"), &user1));

    //external update to CmsAddress
    execute_statement(
        &em,
        "update cms_addresses set user_id = ?",
        php_array![get(&user2, "id")],
    )
    .await;

    // But we want to have this external change!
    // Solution 1: refresh(), broken atm!
    em.refresh(&address).await.unwrap();

    // Now the association should be "correct", referencing $user2
    assert!(same_value(&get(&address, "user"), &user2));
    assert!(same_value(&get(&user2, "address"), &address)); // check back reference also

    // Attention! refreshes can result in broken bidirectional associations! this is currently expected!
    // $user1 still points to $address!
    assert!(same_value(&get(&user1, "address"), &address));
}

#[tokio::test]
async fn testSingleValuedAssociationIdentityMapBehaviorWithRefreshQuery() {
    let em = set_up().await;
    let (address, user1, user2) = address_with_user1_user2();

    em.persist(&address).unwrap();
    em.persist(&user1).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();

    assert!(same_value(&get(&address, "user"), &user1));

    //external update to CmsAddress
    execute_statement(
        &em,
        "update cms_addresses set user_id = ?",
        php_array![get(&user2, "id")],
    )
    .await;

    //select
    let dql = "select a, u from Doctrine\\Tests\\Models\\CMS\\CmsAddress a join a.user u";
    let address2 = get_single_result(&em, dql, false).await;

    assert!(same(&address, &address2));

    // Should still be $user1
    assert!(same_value(&get(&address2, "user"), &user1));
    assert!(get(&user2, "address").is_null());

    // But we want to have this external change!
    // Solution 2: Alternatively, a refresh query should work
    let address3 = get_single_result(&em, dql, true).await;

    assert!(same(&address, &address3)); // should still be the same, always from identity map

    // Now the association should be "correct", referencing $user2
    assert!(same_value(&get(&address2, "user"), &user2));
    assert!(same_value(&get(&user2, "address"), &address2)); // check back reference also

    // Attention! refreshes can result in broken bidirectional associations! this is currently expected!
    // $user1 still points to $address2!
    assert!(same_value(&get(&user1, "address"), &address2));
}

fn user_with_three_phonenumbers() -> Object {
    let user = cms_user_with("Roman B.", "romanb", "dev");

    for number in [123, 234, 345] {
        let phone = cms_phonenumber("");
        set(&phone, "phonenumber", Value::Int(number));
        add_phonenumber(&user, &phone);
    }
    user
}

#[tokio::test]
async fn testCollectionValuedAssociationIdentityMapBehaviorWithRefreshQuery() {
    let em = set_up().await;
    let user = user_with_three_phonenumbers();

    em.persist(&user).unwrap(); // cascaded to phone numbers
    em.flush().await.unwrap();

    assert_eq!(count_of(&user, "phonenumbers").await, 3);
    let phonenumbers = PersistentCollection::from_value(&get(&user, "phonenumbers")).unwrap();
    assert!(!phonenumbers.is_dirty());

    //external update to CmsAddress
    execute_statement(
        &em,
        "insert into cms_phonenumbers (phonenumber, user_id) VALUES (?,?)",
        php_array![999, get(&user, "id")],
    )
    .await;

    //select
    let dql = "select u, p from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.phonenumbers p";
    let user2 = get_single_result(&em, dql, false).await;

    assert!(same(&user, &user2));

    // Should still be the same 3 phonenumbers
    assert_eq!(count_of(&user2, "phonenumbers").await, 3);

    // But we want to have this external change!
    // Solution 1: refresh().
    //$this->_em->refresh($user2); broken atm!
    // Solution 2: Alternatively, a refresh query should work
    let user3 = get_single_result(&em, dql, true).await;

    assert!(same(&user, &user3)); // should still be the same, always from identity map

    // Now the collection should be refreshed with correct count
    assert_eq!(count_of(&user3, "phonenumbers").await, 4);
}

#[tokio::test]
async fn testCollectionValuedAssociationIdentityMapBehaviorWithRefresh() {
    let em = set_up().await;
    let user = user_with_three_phonenumbers();

    em.persist(&user).unwrap(); // cascaded to phone numbers
    em.flush().await.unwrap();

    assert_eq!(count_of(&user, "phonenumbers").await, 3);

    //external update to CmsAddress
    execute_statement(
        &em,
        "insert into cms_phonenumbers (phonenumber, user_id) VALUES (?,?)",
        php_array![999, get(&user, "id")],
    )
    .await;

    //select
    let dql = "select u, p from Doctrine\\Tests\\Models\\CMS\\CmsUser u join u.phonenumbers p";
    let user2 = get_single_result(&em, dql, false).await;

    assert!(same(&user, &user2));

    // Should still be the same 3 phonenumbers
    assert_eq!(count_of(&user2, "phonenumbers").await, 3);

    // But we want to have this external change!
    // Solution 1: refresh().
    em.refresh(&user2).await.unwrap();

    assert!(same(&user, &user2)); // should still be the same, always from identity map

    // Now the collection should be refreshed with correct count
    assert_eq!(count_of(&user2, "phonenumbers").await, 4);
}
