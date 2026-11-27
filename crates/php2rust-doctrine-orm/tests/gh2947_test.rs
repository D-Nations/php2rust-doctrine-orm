//! Port of tests/Tests/ORM/Functional/Ticket/GH2947Test.php, on `SQLite`
//! with a logging connection (`tests/common/query_log.rs`). The test's
//! `$this->resultCache` is the configuration's result cache;
//! `(string) $car` is its `brand` (`__toString()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH2947Test.php

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_dbal::cache::ArrayAdapter;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::Query;

use orm::{get, obj, s, set};
use query_log::QueryLog;
use ticket::{class, new};

async fn set_up() -> (Arc<EntityManager>, Arc<QueryLog>) {
    let car = class("GH2947Car");
    let (em, log) = query_log::set_up_logged(&[car.as_str()]).await;
    em.get_configuration()
        .set_result_cache(Arc::new(ArrayAdapter::new()));
    (em, log)
}

fn assert_query_count(log: &QueryLog, expected: usize) {
    assert_eq!(log.queries().len(), expected);
}

/// `(string) $query->getSingleResult()`.
async fn single_brand(query: &Query) -> Value {
    get(&obj(&query.get_single_result(None).await.unwrap()), "brand")
}

#[tokio::test]
async fn testIssue() {
    let (em, log) = set_up().await;
    create_data(&em).await;
    log.reset_and_enable();

    let query = create_query(&em);
    assert_eq!(single_brand(&query).await, s("BMW"));
    assert_query_count(&log, 1);

    update_data(&em).await;
    assert_eq!(single_brand(&query).await, s("BMW"));
    assert_query_count(&log, 2);

    query.expire_result_cache(true);
    assert_eq!(single_brand(&query).await, s("Dacia"));
    assert_query_count(&log, 3);

    query.expire_result_cache(false);
    assert_eq!(single_brand(&query).await, s("Dacia"));
    assert_query_count(&log, 3);
}

fn create_query(em: &Arc<EntityManager>) -> Query {
    let mut qb = em.create_query_builder();
    let car = class("GH2947Car");
    qb.select(vec![s("car")])
        .unwrap()
        .from(&car, "car", None)
        .unwrap();
    let query = qb.get_query().unwrap();
    query.enable_result_cache(Some(3600), Some("foo-cache-id"));
    query
}

async fn create_data(em: &Arc<EntityManager>) {
    let car = new("GH2947Car");
    set(&car, "brand", s("BMW"));
    em.persist(&car).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

async fn update_data(em: &Arc<EntityManager>) {
    let mut qb = em.create_query_builder();
    let car = class("GH2947Car");
    qb.update(Some(&car), Some("car"))
        .unwrap()
        .set("car.brand", s(":newBrand"))
        .unwrap()
        .r#where(vec![s("car.brand = :oldBrand")])
        .unwrap()
        .set_parameter("newBrand", s("Dacia"), None)
        .set_parameter("oldBrand", s("BMW"), None);
    qb.get_query().unwrap().execute(None, None).await.unwrap();
}
