//! Functional/QueryBuilderParenthesisTest.php on `SQLite`
//! (`tests/common/orm.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/QueryBuilderParenthesisTest.php

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

use orm::s;

const ENTITY: &str = "Doctrine\\Tests\\ORM\\Functional\\QueryBuilderParenthesisEntity";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[ENTITY]).await
}

fn count(value: &Value) -> usize {
    match value {
        Value::Array(array) => array.len(),
        other => panic!("not an array: {other:?}"),
    }
}

#[tokio::test]
async fn testParenthesisOnSingleLine() {
    let em = set_up().await;
    let mut query_builder = em.create_query_builder();
    query_builder
        .select(vec![s("o")])
        .unwrap()
        .from(ENTITY, "o", None)
        .unwrap();
    query_builder
        .and_where(vec![s("o.property3 = :value3")])
        .unwrap()
        .set_parameter("value3", "x", None);
    query_builder
        .and_where(vec![s("o.property1 = :value1 OR o.property2 = :value2")])
        .unwrap();
    query_builder
        .and_where(vec![s("o.property1 = :value1 or o.property2 = :value2")])
        .unwrap();
    query_builder.set_parameter("value1", "x", None);
    query_builder.set_parameter("value2", "x", None);

    let query = query_builder.get_query().unwrap();
    let results = query.get_result(HYDRATE_OBJECT).await.unwrap();
    assert_eq!(count(&results), 0);

    let dql = query.get_dql();

    assert_eq!(
        dql.as_deref(),
        Some(
            format!(
                "SELECT o FROM {ENTITY} o WHERE o.property3 = :value3 AND (o.property1 = :value1 OR o.property2 = :value2) AND (o.property1 = :value1 or o.property2 = :value2)"
            )
            .as_str()
        ),
    );
}

#[tokio::test]
async fn testParenthesisOnMultiLine() {
    let em = set_up().await;
    let mut query_builder = em.create_query_builder();
    query_builder
        .select(vec![s("o")])
        .unwrap()
        .from(ENTITY, "o", None)
        .unwrap();
    query_builder
        .and_where(vec![s("o.property3 = :value3")])
        .unwrap()
        .set_parameter("value3", "x", None);

    query_builder
        .and_where(vec![s("o.property1 = :value1\nOR o.property2 = :value2")])
        .unwrap();
    query_builder.set_parameter("value1", "x", None);
    query_builder.set_parameter("value2", "x", None);

    let query = query_builder.get_query().unwrap();
    let results = query.get_result(HYDRATE_OBJECT).await.unwrap();
    assert_eq!(count(&results), 0);

    let dql = query.get_dql();

    assert_eq!(
        dql.as_deref(),
        Some(
            format!(
                "SELECT o FROM {ENTITY} o WHERE o.property3 = :value3 AND (o.property1 = :value1\nOR o.property2 = :value2)"
            )
            .as_str()
        ),
    );
}
