//! Functional/QueryIterableTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/QueryIterableTest.php
//! Upstream: doctrine/orm tests/Tests/IterableTester.php

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
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::HYDRATE_OBJECT;

use orm::{
    CMS_ARTICLE, CMS_MODELS, CMS_USER, assert_results_are_the_same, cms_address, cms_article,
    cms_user_with, get, obj, s, set,
};

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&CMS_MODELS).await
}

/// `$users[0]['user']->username`.
fn first_user_username(users: &Value) -> Value {
    let Value::Array(users) = users else {
        panic!("not a list: {users:?}");
    };
    assert_eq!(users.len(), 1);
    let Some(Value::Array(row)) = users.values().next() else {
        panic!("not a row: {users:?}");
    };
    get(&obj(&row.get(&"user".into()).cloned().unwrap()), "username")
}

#[tokio::test]
async fn testAlias() {
    let em = set_up().await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query("SELECT u AS user FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u");

    let users = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(first_user_username(&users), s("gblanco"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testAliasInnerJoin() {
    let em = set_up().await;
    let user = cms_user_with("Guilherme", "gblanco", "developer");

    let address = cms_address("Germany", "12345", "Berlin");

    set(&address, "user", Value::Object(user.clone()));
    set(&user, "address", Value::Object(address.clone()));

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(
        "SELECT u AS user, a AS address FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u JOIN u.address a",
    );

    let users = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(first_user_username(&users), s("gblanco"));

    em.clear().unwrap();

    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testIndexByQueryWithOneResult() {
    let em = set_up().await;
    let user = cms_user_with("Antonio J.", "ajgarlag", "developer");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(&format!("SELECT u FROM {CMS_USER} u INDEX BY u.username"));
    assert_results_are_the_same(&query).await;
}

#[tokio::test]
async fn testIndexByQueryWithMultipleResults() {
    let em = set_up().await;
    let article1 = cms_article("Doctrine 2", "This is an introduction to Doctrine 2.");

    let article2 = cms_article("Symfony 2", "This is an introduction to Symfony 2.");

    let article3 = cms_article("Laminas", "This is an introduction to Laminas.");

    let article4 = cms_article("CodeIgniter", "This is an introduction to CodeIgniter.");

    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();
    em.persist(&article3).unwrap();
    em.persist(&article4).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!("select a from {CMS_ARTICLE} a INDEX BY a.topic"));
    assert_results_are_the_same(&query).await;
}
