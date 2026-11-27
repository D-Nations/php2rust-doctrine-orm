//! Port of `ProxiesLikeEntitiesTest`, on `SQLite`: test that Doctrine ORM
//! correctly works with proxy instances exactly like with ordinary
//! Entities. A write to an uninitialized ghost's property initializes it
//! first in PHP: `initialize_object()` is called before it here.
//! `tearDown()`'s `DELETE` is not
//! needed, as each test has its own database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ProxiesLikeEntitiesTest.php

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
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{CMS, CMS_USER, assert_loose_eq, cms_user, get, obj, s, set};

/// `setUp()`: the schema and a persisted user.
async fn set_up() -> (Arc<EntityManager>, Object) {
    let classes = [
        "CmsUser",
        "CmsTag",
        "CmsPhonenumber",
        "CmsArticle",
        "CmsAddress",
        "CmsEmail",
        "CmsGroup",
    ]
    .map(|c| format!("{CMS}{c}"));
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let em = orm::set_up(&classes).await;

    let user = cms_user();
    set(&user, "username", s("ocramius"));
    set(&user, "name", s("Marco"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    (em, user)
}

fn id_criteria(id: Value) -> PhpArray {
    let mut criteria = PhpArray::new();
    criteria.insert("id", id);
    criteria
}

/// Verifies that a proxy can be successfully persisted and updated
#[tokio::test]
async fn testPersistUpdate() {
    let (em, user) = set_up().await;
    // Considering case (a)
    let proxy = em
        .get_proxy_factory()
        .get_proxy(CMS_USER, &id_criteria(get(&user, "id")))
        .unwrap();

    set(&proxy, "id", Value::Null);
    // The write to `username` initializes the ghost first.
    em.initialize_object(&proxy).await.unwrap();
    set(&proxy, "username", s("ocra"));
    set(&proxy, "name", s("Marco"));
    em.persist(&proxy).unwrap();
    em.flush().await.unwrap();
    assert!(!get(&proxy, "id").is_null());
    set(&proxy, "name", s("Marco Pivetta"));
    em.get_unit_of_work()
        .compute_change_set(&em.get_class_metadata(CMS_USER).unwrap(), &proxy)
        .await
        .unwrap();
    assert!(
        !em.get_unit_of_work()
            .get_entity_change_set(&proxy)
            .is_empty()
    );
    let found = em.find(CMS_USER, get(&proxy, "id")).await.unwrap().unwrap();
    assert_loose_eq(&s("Marco Pivetta"), &get(&found, "name"));
    em.remove(&proxy).await.unwrap();
    em.flush().await.unwrap();
}

#[tokio::test]
async fn testEntityWithIdentifier() {
    let (em, user) = set_up().await;
    let user_id = get(&user, "id");
    let uninitialized_proxy = em
        .get_reference(CMS_USER, user_id.clone())
        .await
        .unwrap()
        .unwrap();
    let proxy_value = Value::Object(uninitialized_proxy.clone());
    assert!(em.is_uninitialized_object(&proxy_value));

    em.persist(&uninitialized_proxy).unwrap();
    em.flush().await.unwrap();
    assert!(
        em.is_uninitialized_object(&proxy_value),
        "Proxy didn't get initialized during flush operations"
    );
    assert_loose_eq(&user_id, &get(&uninitialized_proxy, "id"));
    em.remove(&uninitialized_proxy).await.unwrap();
    em.flush().await.unwrap();
}

/// Verifying that proxies can be used without problems as query parameters
#[tokio::test]
async fn testProxyAsDqlParameterPersist() {
    let (em, user) = set_up().await;
    let proxy = em
        .get_reference(CMS_USER, Value::Array(id_criteria(get(&user, "id"))))
        .await
        .unwrap()
        .unwrap();
    set(&proxy, "id", get(&user, "id"));
    let query =
        em.create_query("SELECT u FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u WHERE u = ?1");
    query.set_parameter(1, Value::Object(proxy.clone()), None);
    let result = obj(&query.get_single_result(None).await.unwrap());
    assert!(get(&user, "id").identical(&get(&result, "id")));
    em.remove(&proxy).await.unwrap();
    em.flush().await.unwrap();
}
