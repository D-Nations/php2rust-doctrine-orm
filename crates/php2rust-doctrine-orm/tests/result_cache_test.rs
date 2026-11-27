//! Port of tests/Tests/ORM/Functional/ResultCacheTest.php, on `SQLite`
//! with a logging connection (`tests/common/query_log.rs`).
//!
//! `#[Depends('testNativeQueryResultCaching')]`: each dependent test runs
//! `testNativeQueryResultCaching()`'s body first and goes on with its
//! query and cache.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ResultCacheTest.php

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
use php2rust_doctrine_orm::native_query::NativeQuery;
use php2rust_doctrine_orm::query::{ResultSetMapping, context::HYDRATE_ARRAY};

use orm::{CMS_MODELS, cms_article, cms_user_with, get, list, obj, s, set_obj};
use query_log::QueryLog;

struct Fixture {
    em: Arc<EntityManager>,
    log: Arc<QueryLog>,
}

async fn set_up() -> Fixture {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;
    Fixture { em, log }
}

impl Fixture {
    /// `assertQueryCount($expected)`.
    fn assert_query_count(&self, expected: usize, message: &str) {
        assert_eq!(self.log.queries().len(), expected, "{message}");
    }

    /// `$this->_em->getConfiguration()->setResultCache(new ArrayAdapter())`.
    fn reset_cache(&self) {
        self.em
            .get_configuration()
            .set_result_cache(Arc::new(ArrayAdapter::new()));
    }
}

/// `setResultCache($query, $cache)`.
macro_rules! set_result_cache {
    ($query:expr, $cache:expr) => {
        $query.set_result_cache_profile(Some(
            QueryCacheProfile::default().set_result_cache($cache.clone()),
        ))
    };
}

fn assert_cache_has_item(key: &str, cache: &ArrayAdapter) {
    assert!(
        cache.has_item(key).unwrap(),
        "Failed asserting that a given cache contains the key \"{key}\"."
    );
}

fn assert_cache_does_not_have_item(key: &str, cache: &ArrayAdapter) {
    assert!(
        !cache.has_item(key).unwrap(),
        "Failed asserting that a given cache does not contain the key \"{key}\"."
    );
}

fn pool(cache: &Arc<ArrayAdapter>) -> Arc<dyn CacheItemPool> {
    cache.clone()
}

const SELECT_USERS: &str = "select ux from Doctrine\\Tests\\Models\\CMS\\CmsUser ux";
const SELECT_USER_BY_ID: &str =
    "select ux from Doctrine\\Tests\\Models\\CMS\\CmsUser ux WHERE ux.id = ?1";

#[tokio::test]
async fn testResultCache() {
    let f = set_up().await;
    let user = cms_user_with("Roman", "romanb", "dev");

    f.em.persist(&user).unwrap();
    f.em.flush().await.unwrap();

    let query = f.em.create_query(SELECT_USERS);

    let cache = Arc::new(ArrayAdapter::new());

    set_result_cache!(query, pool(&cache));
    query.set_result_cache_id(Some("my_cache_id"));

    assert_cache_does_not_have_item("my_cache_id", &cache);

    let users = list(&query.get_object_result().await.unwrap());

    assert_cache_has_item("my_cache_id", &cache);
    assert_eq!(users.len(), 1);
    assert_eq!(get(&obj(&users[0]), "name"), s("Roman"));

    f.em.clear().unwrap();

    let query2 = f.em.create_query(SELECT_USERS);
    set_result_cache!(query2, pool(&cache));
    query2.set_result_cache_id(Some("my_cache_id"));

    let users = list(&query2.get_object_result().await.unwrap());

    assert_cache_has_item("my_cache_id", &cache);
    assert_eq!(users.len(), 1);
    assert_eq!(get(&obj(&users[0]), "name"), s("Roman"));
}

#[tokio::test]
async fn testSetResultCacheId() {
    let f = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());
    let query = f.em.create_query(SELECT_USERS);

    set_result_cache!(query, pool(&cache));
    query.set_result_cache_id(Some("testing_result_cache_id"));

    assert_cache_does_not_have_item("testing_result_cache_id", &cache);

    query.get_object_result().await.unwrap();

    assert_cache_has_item("testing_result_cache_id", &cache);
}

/// DDC-1026
#[tokio::test]
async fn testUseResultCacheParams() {
    let f = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());
    f.log.reset_and_enable();
    let query = f.em.create_query(SELECT_USER_BY_ID);

    set_result_cache!(query, pool(&cache));

    // these queries should result in cache miss:
    query.set_parameter(1, Value::Int(1), None);
    query.get_object_result().await.unwrap();
    query.set_parameter(1, Value::Int(2), None);
    query.get_object_result().await.unwrap();

    f.assert_query_count(2, "Two non-cached queries.");

    // these two queries should actually be cached, as they repeat previous ones:
    query.set_parameter(1, Value::Int(1), None);
    query.get_object_result().await.unwrap();
    query.set_parameter(1, Value::Int(2), None);
    query.get_object_result().await.unwrap();

    f.assert_query_count(
        2,
        "The next two sql queries should have been cached, but were not.",
    );
}

#[tokio::test]
async fn testEnableResultCache() {
    let f = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());
    let query = f.em.create_query(SELECT_USERS);

    query.enable_result_cache(None, None);
    set_result_cache!(query, pool(&cache));
    query.set_result_cache_id(Some("testing_result_cache_id"));
    query.get_object_result().await.unwrap();

    assert_cache_has_item("testing_result_cache_id", &cache);

    f.reset_cache();
}

#[tokio::test]
async fn testEnableResultCacheWithIterable() {
    let f = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());
    f.log.reset_and_enable();

    let query = f.em.create_query(SELECT_USERS);
    query.enable_result_cache(None, None);
    set_result_cache!(query, pool(&cache));
    query.set_result_cache_id(Some("testing_iterable_result_cache_id"));
    query
        .to_iterable(None, php2rust_doctrine_orm::query::context::HYDRATE_OBJECT)
        .await
        .unwrap();

    f.em.clear().unwrap();

    f.assert_query_count(1, "");
    assert_cache_has_item("testing_iterable_result_cache_id", &cache);

    let query = f.em.create_query(SELECT_USERS);
    query.enable_result_cache(None, None);
    set_result_cache!(query, pool(&cache));
    query.set_result_cache_id(Some("testing_iterable_result_cache_id"));
    query
        .to_iterable(None, php2rust_doctrine_orm::query::context::HYDRATE_OBJECT)
        .await
        .unwrap();

    f.assert_query_count(1, "Expected query to be cached");

    f.reset_cache();
}

/// DDC-1026
#[tokio::test]
async fn testEnableResultCacheParams() {
    let f = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());
    f.log.reset_and_enable();
    let query = f.em.create_query(SELECT_USER_BY_ID);

    set_result_cache!(query, pool(&cache));
    query.enable_result_cache(None, None);

    // these queries should result in cache miss:
    query.set_parameter(1, Value::Int(1), None);
    query.get_object_result().await.unwrap();
    query.set_parameter(1, Value::Int(2), None);
    query.get_object_result().await.unwrap();

    f.assert_query_count(2, "Two non-cached queries.");

    // these two queries should actually be cached, as they repeat previous ones:
    query.set_parameter(1, Value::Int(1), None);
    query.get_object_result().await.unwrap();
    query.set_parameter(1, Value::Int(2), None);
    query.get_object_result().await.unwrap();

    f.assert_query_count(
        2,
        "The next two sql queries should have been cached, but were not.",
    );
}

#[tokio::test]
async fn testDisableResultCache() {
    let f = set_up().await;
    let cache = Arc::new(ArrayAdapter::new());
    let query = f.em.create_query(SELECT_USERS);

    set_result_cache!(query, pool(&cache));
    query.set_result_cache_id(Some("testing_result_cache_id"));
    query.disable_result_cache();
    query.get_object_result().await.unwrap();

    assert!(!cache.has_item("testing_result_cache_id").unwrap());

    f.reset_cache();
}

/// `testNativeQueryResultCaching()`: its query and cache, for the tests
/// that depend on it.
async fn native_query_result_caching(f: &Fixture) -> (NativeQuery, Arc<ArrayAdapter>) {
    let cache = Arc::new(ArrayAdapter::new());
    let mut rsm = ResultSetMapping::new();

    rsm.add_scalar_result("id", "u", "integer");

    let query =
        f.em.create_native_query("select u.id FROM cms_users u WHERE u.id = ?", rsm)
            .unwrap();

    query.set_parameter(1, Value::Int(10), None);
    set_result_cache!(query, pool(&cache));
    query.enable_result_cache(None, None);

    assert!(cache.get_values().is_empty());

    query.get_object_result().await.unwrap();

    assert!(!cache.get_values().is_empty());

    (query, cache)
}

#[tokio::test]
async fn testNativeQueryResultCaching() {
    let f = set_up().await;
    native_query_result_caching(&f).await;
}

#[tokio::test]
async fn testResultCacheNotDependsOnQueryHints() {
    let f = set_up().await;
    let (query, cache) = native_query_result_caching(&f).await;

    let cache_count = cache.get_values().len();

    query.set_hint(
        "foo",
        php2rust_doctrine_orm::query::context::Hint::Value(s("bar")),
    );
    query.get_object_result().await.unwrap();

    assert_eq!(cache.get_values().len(), cache_count);
}

#[tokio::test]
async fn testResultCacheDependsOnParameters() {
    let f = set_up().await;
    let (query, cache) = native_query_result_caching(&f).await;

    let cache_count = cache.get_values().len();

    query.set_parameter(1, Value::Int(50), None);
    query.get_object_result().await.unwrap();

    assert_eq!(cache.get_values().len(), cache_count + 1);
}

#[tokio::test]
async fn testResultCacheNotDependsOnHydrationMode() {
    let f = set_up().await;
    let (query, cache) = native_query_result_caching(&f).await;

    let cache_count = cache.get_values().len();

    assert_ne!(query.get_hydration_mode(), HYDRATE_ARRAY);
    query.get_array_result().await.unwrap();

    assert_eq!(cache.get_values().len(), cache_count);
}

/// DDC-909
#[tokio::test]
async fn testResultCacheWithObjectParameter() {
    let f = set_up().await;
    let user1 = cms_user_with("Roman", "romanb", "dev");
    let user2 = cms_user_with("Benjamin", "beberlei", "dev");

    let article = cms_article("baz", "foo");
    set_obj(&article, "user", &user1);

    f.em.persist(&article).unwrap();
    f.em.persist(&user1).unwrap();
    f.em.persist(&user2).unwrap();
    f.em.flush().await.unwrap();

    let select = "select a from Doctrine\\Tests\\Models\\CMS\\CmsArticle a WHERE a.user = ?1";
    let query = f.em.create_query(select);
    query.set_parameter(1, Value::Object(user1.clone()), None);

    let cache = Arc::new(ArrayAdapter::new());

    set_result_cache!(query, pool(&cache));
    query.enable_result_cache(None, None);

    let articles = list(&query.get_object_result().await.unwrap());

    assert_eq!(articles.len(), 1);
    assert_eq!(get(&obj(&articles[0]), "topic"), s("baz"));

    f.em.clear().unwrap();

    let query2 = f.em.create_query(select);
    query2.set_parameter(1, Value::Object(user1.clone()), None);

    set_result_cache!(query2, pool(&cache));
    query2.enable_result_cache(None, None);

    let articles = list(&query2.get_object_result().await.unwrap());

    assert_eq!(articles.len(), 1);
    assert_eq!(get(&obj(&articles[0]), "topic"), s("baz"));

    let query3 = f.em.create_query(select);
    query3.set_parameter(1, Value::Object(user2.clone()), None);

    set_result_cache!(query3, pool(&cache));
    query3.enable_result_cache(None, None);

    let articles = list(&query3.get_object_result().await.unwrap());

    assert_eq!(articles.len(), 0);
}
