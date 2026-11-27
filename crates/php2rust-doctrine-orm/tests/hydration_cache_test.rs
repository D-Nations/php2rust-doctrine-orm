//! Port of tests/Tests/ORM/Functional/HydrationCacheTest.php, on `SQLite`
//! with a logging connection (`tests/common/query_log.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/HydrationCacheTest.php

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

use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_dbal::cache::{ArrayAdapter, CacheItemPool, QueryCacheProfile};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{CMS_MODELS, cms_user_with};
use query_log::QueryLog;

async fn set_up() -> (Arc<EntityManager>, Arc<QueryLog>) {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;

    let user = cms_user_with("Benjamin", "beberlei", "active");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    (em, log)
}

fn assert_query_count(log: &QueryLog, expected: usize, message: &str) {
    assert_eq!(log.queries().len(), expected, "{message}");
}

fn profile(key: Option<&str>, cache: &Arc<ArrayAdapter>) -> QueryCacheProfile {
    let pool: Arc<dyn CacheItemPool> = cache.clone();
    QueryCacheProfile::new(0, key.map(str::to_owned), Some(pool))
}

#[tokio::test]
async fn testHydrationCache() {
    let (em, log) = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());

    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\Cms\\CmsUser u";

    em.create_query(dql)
        .set_hydration_cache_profile(Some(profile(None, &cache)))
        .get_object_result()
        .await
        .unwrap();

    log.reset_and_enable();
    em.create_query(dql)
        .set_hydration_cache_profile(Some(profile(None, &cache)))
        .get_object_result()
        .await
        .unwrap();

    assert_query_count(&log, 0, "Should not execute query. Its cached!");

    em.create_query(dql)
        .set_hydration_cache_profile(Some(profile(None, &cache)))
        .get_array_result()
        .await
        .unwrap();

    assert_query_count(&log, 1, "Hydration is part of cache key.");

    em.create_query(dql)
        .set_hydration_cache_profile(Some(profile(None, &cache)))
        .get_array_result()
        .await
        .unwrap();

    assert_query_count(&log, 1, "Hydration now cached");

    em.create_query(dql)
        .set_hydration_cache_profile(Some(profile(Some("cachekey"), &cache)))
        .get_array_result()
        .await
        .unwrap();

    assert!(cache.has_item("cachekey").unwrap(), "Explicit cache key");

    em.create_query(dql)
        .set_hydration_cache_profile(Some(profile(Some("cachekey"), &cache)))
        .get_array_result()
        .await
        .unwrap();
    assert_query_count(&log, 2, "Hydration now cached");
}

#[tokio::test]
async fn testHydrationParametersSerialization() {
    let (em, log) = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());

    let dql = "SELECT u FROM Doctrine\\Tests\\Models\\Cms\\CmsUser u WHERE u.id = ?1";
    let query = em.create_query(dql);
    query
        .set_parameter(1, Value::Int(1), None)
        .set_hydration_cache_profile(Some(profile(None, &cache)));

    query.get_object_result().await.unwrap();

    log.reset_and_enable();

    query.get_object_result().await.unwrap();

    assert_query_count(&log, 0, "Should not execute query. Its cached!");
}
