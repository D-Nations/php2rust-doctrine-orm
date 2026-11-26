//! Functional/QueryParameterTest.php on `SQLite` (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/QueryParameterTest.php

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

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_dbal::{ArrayParameterType, ParameterType, WrapperParameterType};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query_builder::QueryBuilder;

use orm::{CMS_MODELS, CMS_USER, cms_user, get, s, set};

struct Fixture {
    em: Arc<EntityManager>,
    user_id: Value,
}

async fn set_up() -> Fixture {
    let em = orm::set_up(&CMS_MODELS).await;

    let user = cms_user();
    set(&user, "name", s("John Doe"));
    set(&user, "username", s("john"));
    let user2 = cms_user();
    set(&user2, "name", s("Jane Doe"));
    set(&user2, "username", s("jane"));
    let user3 = cms_user();
    set(&user3, "name", s("Just Bill"));
    set(&user3, "username", s("bill"));

    em.persist(&user).unwrap();
    em.persist(&user2).unwrap();
    em.persist(&user3).unwrap();
    em.flush().await.unwrap();

    let user_id = get(&user, "id");

    em.clear().unwrap();

    Fixture { em, user_id }
}

/// `[['name' => $name], ...]`.
fn names(names: &[&str]) -> Value {
    let mut rows = PhpArray::new();
    for name in names {
        rows.push(Value::Array(php_array! { "name" => *name }))
            .unwrap();
    }
    Value::Array(rows)
}

fn assert_same(expected: &Value, actual: &Value) {
    assert!(expected.identical(actual), "{expected:?} !== {actual:?}");
}

/// `->from(CmsUser::class, 'u')->select('u.name')->where($where)`.
fn builder(em: &Arc<EntityManager>, r#where: &str) -> QueryBuilder {
    let mut qb = em.create_query_builder();
    qb.from(CMS_USER, "u", None)
        .unwrap()
        .select(vec![s("u.name")])
        .unwrap()
        .r#where(vec![s(r#where)])
        .unwrap();
    qb
}

fn integer() -> WrapperParameterType {
    WrapperParameterType::Parameter(ParameterType::Integer)
}

fn dbal_integer() -> WrapperParameterType {
    WrapperParameterType::Name(Types::INTEGER.into())
}

fn string_list() -> WrapperParameterType {
    WrapperParameterType::Array(ArrayParameterType::String)
}

#[tokio::test]
async fn testParameterTypeInBuilder() {
    let f = set_up().await;
    let mut qb = builder(&f.em, "u.id = :id");
    qb.set_parameter("id", f.user_id.clone(), Some(integer()));
    let result = qb.get_query().unwrap().get_array_result().await.unwrap();

    assert_same(&names(&["John Doe"]), &result);
}

#[tokio::test]
async fn testParameterTypeInQuery() {
    let f = set_up().await;
    let query = builder(&f.em, "u.id = :id").get_query().unwrap();
    query.set_parameter("id", f.user_id.clone(), Some(integer()));
    let result = query.get_array_result().await.unwrap();

    assert_same(&names(&["John Doe"]), &result);
}

#[tokio::test]
async fn testDbalTypeStringInBuilder() {
    let f = set_up().await;
    let mut qb = builder(&f.em, "u.id = :id");
    qb.set_parameter("id", f.user_id.clone(), Some(dbal_integer()));
    let result = qb.get_query().unwrap().get_array_result().await.unwrap();

    assert_same(&names(&["John Doe"]), &result);
}

#[tokio::test]
async fn testDbalTypeStringInQuery() {
    let f = set_up().await;
    let query = builder(&f.em, "u.id = :id").get_query().unwrap();
    query.set_parameter("id", f.user_id.clone(), Some(dbal_integer()));
    let result = query.get_array_result().await.unwrap();

    assert_same(&names(&["John Doe"]), &result);
}

#[tokio::test]
async fn testArrayParameterTypeInBuilder() {
    let f = set_up().await;
    let mut qb = builder(&f.em, "u.username IN (:usernames)");
    qb.order_by("u.username", None).unwrap();
    qb.set_parameter("usernames", php_array!["john", "jane"], Some(string_list()));
    let result = qb.get_query().unwrap().get_array_result().await.unwrap();

    assert_same(&names(&["Jane Doe", "John Doe"]), &result);
}

#[tokio::test]
async fn testArrayParameterTypeInQuery() {
    let f = set_up().await;
    let mut qb = builder(&f.em, "u.username IN (:usernames)");
    qb.order_by("u.username", None).unwrap();
    let query = qb.get_query().unwrap();
    query.set_parameter("usernames", php_array!["john", "jane"], Some(string_list()));
    let result = query.get_array_result().await.unwrap();

    assert_same(&names(&["Jane Doe", "John Doe"]), &result);
}
