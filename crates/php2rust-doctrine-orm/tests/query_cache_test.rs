//! Upstream: doctrine/orm tests/Tests/ORM/Functional/QueryCacheTest.php
//!
//! A `#[Depends('testQueryCacheDependsOnHints')]` test runs that test
//! first and takes its query and cache. `createMock(CacheItemPoolInterface)`
//! is a pool that records its `getItem()` keys and saved items over an
//! `ArrayAdapter`; a cache item is a value, so the expectations on its
//! `get()` and `set()` are the pool's hit and saved items. The
//! `AbstractSqlExecutor` stub, which returns 10, is an `UPDATE` of no rows
//! (an executor is data here); the test checks that the cached parser
//! result's finalizer made it.

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

use std::sync::{Arc, Mutex};

use php_value::{Thrown, Value};
use php2rust_doctrine_orm::cache::pool::{
    ArrayAdapter, CacheItem, CacheItemPool, CachedParserResult, PoolValue,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::ParserResult;
use php2rust_doctrine_orm::query::context::{DqlQuery, Hint};
use php2rust_doctrine_orm::query::dql_query::Query;
use php2rust_doctrine_orm::query::exec::{ExecutorKind, SqlExecutor, SqlFinalizer, SqlStatements};

use orm::CMS_MODELS;

const DQL: &str = "select ux from Doctrine\\Tests\\Models\\CMS\\CmsUser ux";

/// `setUp()`.
async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&CMS_MODELS).await
}

/// `createMock(CacheItemPoolInterface::class)`.
#[derive(Debug, Default)]
struct PoolMock {
    inner: ArrayAdapter,
    /// The item `getItem()` returns, whatever the key (`willReturn()`).
    returned_key: Option<String>,
    /// The key of each `getItem()`.
    get_item_keys: Mutex<Vec<String>>,
    /// The item of each `save()`.
    saved: Mutex<Vec<CacheItem>>,
}

impl CacheItemPool for PoolMock {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown> {
        self.get_item_keys.lock().unwrap().push(key.to_owned());
        self.inner
            .get_item(self.returned_key.as_deref().unwrap_or(key))
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown> {
        self.inner.get_items(keys)
    }

    fn has_item(&self, key: &str) -> Result<bool, Thrown> {
        self.inner.has_item(key)
    }

    fn clear(&self, prefix: &str) -> bool {
        self.inner.clear(prefix)
    }

    fn delete_item(&self, key: &str) -> Result<bool, Thrown> {
        self.inner.delete_item(key)
    }

    fn save(&self, item: CacheItem) -> bool {
        self.saved.lock().unwrap().push(item);
        true
    }
}

async fn query_cache_depends_on_hints(em: &Arc<EntityManager>) -> (Query, Arc<ArrayAdapter>) {
    let query = em.create_query(DQL);

    let cache = Arc::new(ArrayAdapter::new());
    query.set_query_cache(Some(cache.clone()));

    query.get_object_result().await.unwrap();
    assert_eq!(1, cache.get_values().len());

    query.set_hint("foo", Hint::Value(Value::string("bar")));

    query.get_object_result().await.unwrap();
    assert_eq!(2, cache.get_values().len());

    (query, cache)
}

#[tokio::test]
async fn testQueryCacheDependsOnHints() {
    let em = set_up().await;
    query_cache_depends_on_hints(&em).await;
}

#[tokio::test]
async fn testQueryCacheDoesNotDependOnFirstResultForDefaultOutputWalker() {
    let em = set_up().await;
    let (query, cache) = query_cache_depends_on_hints(&em).await;

    let cache_count = cache.get_values().len();

    query.set_first_result(10);
    query.set_max_results(Some(9999));

    query.get_object_result().await.unwrap();
    assert_eq!(cache_count, cache.get_values().len());
}

#[tokio::test]
async fn testQueryCacheDoesNotDependOnMaxResultsForDefaultOutputWalker() {
    let em = set_up().await;
    let (query, cache) = query_cache_depends_on_hints(&em).await;

    let cache_count = cache.get_values().len();

    query.set_max_results(Some(10));

    query.get_object_result().await.unwrap();
    assert_eq!(cache_count, cache.get_values().len());
}

#[tokio::test]
async fn testQueryCacheDependsOnHydrationMode() {
    let em = set_up().await;
    let (query, cache) = query_cache_depends_on_hints(&em).await;

    let cache_count = cache.get_values().len();

    query.get_array_result().await.unwrap();
    assert_eq!(cache_count + 1, cache.get_values().len());
}

#[tokio::test]
async fn testQueryCacheNoHitSaveParserResult() {
    let em = set_up().await;
    em.get_configuration()
        .set_query_cache(Arc::new(ArrayAdapter::new()));

    let query = em.create_query(DQL);

    let cache = Arc::new(PoolMock::default());
    query.set_query_cache(Some(cache.clone()));

    query.get_object_result().await.unwrap();

    // getItem() once, with a string; the item is a miss
    assert_eq!(1, cache.get_item_keys.lock().unwrap().len());
    // save() once, of the item set to a ParserResult
    let saved = cache.saved.lock().unwrap();
    assert_eq!(1, saved.len());
    assert!(!saved[0].is_hit());
    assert!(
        saved[0]
            .get()
            .and_then(PoolValue::downcast_ref::<CachedParserResult>)
            .is_some()
    );
}

/// `createMock(SqlFinalizer::class)`: `createExecutor($query)` returns
/// the executor stub.
#[derive(Debug, Default)]
struct SqlFinalizerMock {
    /// The DQL of each `createExecutor()` query.
    calls: Mutex<Vec<String>>,
}

impl SqlFinalizer for SqlFinalizerMock {
    fn class(&self) -> &'static str {
        "SqlFinalizerMock"
    }

    fn create_executor(&self, query: &dyn DqlQuery) -> Result<SqlExecutor, Thrown> {
        self.calls.lock().unwrap().push(query.get_dql());
        Ok(SqlExecutor::new(
            ExecutorKind::SingleTableDeleteUpdate,
            SqlStatements::One("UPDATE cms_users SET status = status".to_owned()),
        ))
    }
}

/// The finalizer mock, shared with the test.
#[derive(Debug)]
struct Shared(Arc<SqlFinalizerMock>);

impl SqlFinalizer for Shared {
    fn class(&self) -> &str {
        self.0.class()
    }

    fn create_executor(&self, query: &dyn DqlQuery) -> Result<SqlExecutor, Thrown> {
        self.0.create_executor(query)
    }
}

#[tokio::test]
async fn testQueryCacheHitDoesNotSaveParserResult() {
    let em = set_up().await;
    em.get_configuration()
        .set_query_cache(Arc::new(ArrayAdapter::new()));

    let query = em.create_query(DQL);

    let sql_finalizer_mock = Arc::new(SqlFinalizerMock::default());

    let mut parser_result_mock = ParserResult::new();
    parser_result_mock.set_sql_finalizer(Box::new(Shared(sql_finalizer_mock.clone())));

    let cache = PoolMock {
        returned_key: Some("cached".to_owned()),
        ..PoolMock::default()
    };
    // isHit() is true, get() is the parser result
    let item = cache
        .inner
        .get_item("cached")
        .unwrap()
        .set(PoolValue::object(CachedParserResult(Arc::new(
            parser_result_mock,
        ))));
    cache.inner.save(item);
    let cache = Arc::new(cache);

    query.set_query_cache(Some(cache.clone()));

    query.get_object_result().await.unwrap();

    assert_eq!(1, cache.get_item_keys.lock().unwrap().len());
    assert!(cache.saved.lock().unwrap().is_empty());
    assert_eq!(
        vec![DQL.to_owned()],
        *sql_finalizer_mock.calls.lock().unwrap()
    );
}
