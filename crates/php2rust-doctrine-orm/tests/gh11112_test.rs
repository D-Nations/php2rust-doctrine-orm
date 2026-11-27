//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11112Test.php
//!
//! On `SQLite` with the `cms` model set (`tests/common/orm.rs`);
//! `self::$queryCache` is a new `ArrayAdapter` set as the configuration's
//! query cache.

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
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{CMS_MODELS, CMS_USER, s};

async fn set_up() -> (Arc<EntityManager>, Arc<ArrayAdapter>) {
    let query_cache = Arc::new(ArrayAdapter::new());
    let em = orm::set_up(&CMS_MODELS).await;
    em.get_configuration().set_query_cache(query_cache.clone());
    (em, query_cache)
}

#[tokio::test]
async fn testSimpleQueryHasLimitAndOffsetApplied() {
    let (em, query_cache) = set_up().await;
    let platform = em.get_platform();
    let query = em.create_query(&format!("SELECT u FROM {CMS_USER} u"));
    let original_sql = query.get_single_sql().unwrap();

    query.set_max_results(Some(10));
    query.set_first_result(20);
    let sql_max10_first20 = query.get_single_sql().unwrap();

    query.set_max_results(Some(30));
    query.set_first_result(40);
    let sql_max30_first40 = query.get_single_sql().unwrap();

    // The SQL is platform specific and may even be something with outer SELECTS being added. So,
    // derive the expected value at runtime through the platform.
    assert_eq!(
        platform
            .modify_limit_query(&original_sql, Some(10), 20)
            .unwrap(),
        sql_max10_first20
    );
    assert_eq!(
        platform
            .modify_limit_query(&original_sql, Some(30), 40)
            .unwrap(),
        sql_max30_first40
    );

    let cache_entries = query_cache.get_values();
    assert_eq!(1, cache_entries.len());
}

#[tokio::test]
async fn testSubqueryLimitAndOffsetAreIgnored() {
    let (em, _query_cache) = set_up().await;
    // Not sure what to do about this test. Basically, I want to make sure that
    // firstResult/maxResult for subqueries are not relevant, they do not make it
    // into the final query at all. That would give us the guarantee that the
    // "sql finalizer" step is sufficient for the final, "outer" query and we
    // do not need to run finalizers for the subqueries.

    // This DQL/query makes no sense, it's just about creating a subquery in the first place
    let mut subquery = em.create_query_builder();
    subquery.select(vec![s("u")]).unwrap();
    subquery.from(CMS_USER, "u", None).unwrap();
    subquery.set_first_result(Some(10));
    subquery.set_max_results(Some(20)).unwrap();

    let mut query_builder = em.create_query_builder();
    query_builder.select(vec![s("o")]).unwrap();
    query_builder.from(CMS_USER, "o", None).unwrap();
    // The subquery builder is a string (`__toString()`), as is the `Func`.
    let exists = query_builder
        .expr()
        .exists(s(&subquery.get_dql().unwrap()))
        .try_to_string()
        .unwrap();
    query_builder.r#where(vec![Value::String(exists)]).unwrap();

    let query = query_builder.get_query().unwrap();
    let original_sql = query.get_single_sql().unwrap();

    let clone = query.clone_query();
    clone.set_first_result(24);
    clone.set_max_results(Some(42));
    let limited_sql = clone.get_single_sql().unwrap();

    let platform = em.get_platform();

    // The SQL is platform specific and may even be something with outer SELECTS being added. So,
    // derive the expected value at runtime through the platform.
    assert_eq!(
        platform
            .modify_limit_query(&original_sql, Some(42), 24)
            .unwrap(),
        limited_sql
    );
}
