//! Ticket/GH6699Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6699Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::query::Query;
use php2rust_doctrine_orm::query_builder::QueryBuilder;

use orm::*;

/// The query builder of both tests, before its parameters.
fn query_builder(qb: &mut QueryBuilder) {
    qb.select(vec![s("u")])
        .unwrap()
        .from(CMS_USER, "u", None)
        .unwrap()
        .and_where(vec![s("u.username = :username")])
        .unwrap()
        .and_where(vec![s("u.id = ?0")])
        .unwrap();
}

async fn assert_parameters(query: &Query) {
    query.execute(None, None).await.unwrap();

    assert_eq!(2, query.get_parameters().len());
    assert!(
        Value::Int(0).identical(query.get_parameter(0).unwrap().get_value())
    );
    assert!(
        s("bar").identical(query.get_parameter("username").unwrap().get_value())
    );
}

#[tokio::test]
async fn testMixedParametersWithZeroNumber() {
    let em = orm::set_up(&CMS_MODELS).await;
    let mut qb = em.create_query_builder();
    query_builder(&mut qb);
    let query = qb.get_query().unwrap();

    query.set_parameter("username", s("bar"), None);
    query.set_parameter(0, Value::Int(0), None);

    assert_parameters(&query).await;
}

#[tokio::test]
async fn testMixedParametersWithZeroNumberOnQueryBuilder() {
    let em = orm::set_up(&CMS_MODELS).await;
    let mut qb = em.create_query_builder();
    query_builder(&mut qb);
    qb.set_parameter("username", s("bar"), None)
        .set_parameter(0, Value::Int(0), None);
    let query = qb.get_query().unwrap();

    assert_parameters(&query).await;
}
