//! Upstream: doctrine/orm tests/Tests/ORM/Cache/CacheLoggerChainTest.php
//!
//! The `CacheLogger` mock records its calls; `expects(self::once())`
//! with `equalTo()` arguments is one recorded call of the method with
//! equal arguments.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::{Arc, Mutex};

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::cache::logging::{CacheLogger, CacheLoggerChain};
use php2rust_doctrine_orm::cache::{CollectionCacheKey, EntityCacheKey, QueryCacheKey};

const STATE: &str = "Doctrine\\Tests\\Models\\Cache\\State";

/// A recorded call: the method, the region name and the key's hash.
type Call = (&'static str, String, String);

#[derive(Debug, Default)]
struct LoggerMock {
    calls: Mutex<Vec<Call>>,
}

impl LoggerMock {
    fn record(&self, method: &'static str, region_name: &str, hash: &str) {
        self.calls
            .lock()
            .unwrap()
            .push((method, region_name.to_owned(), hash.to_owned()));
    }

    /// `expects(self::once())->method($method)->with($name, $key)`.
    fn assert_once(&self, method: &str, region_name: &str, hash: &str) {
        let calls = self.calls.lock().unwrap();
        let matching: Vec<_> = calls.iter().filter(|(m, _, _)| *m == method).collect();
        assert_eq!(1, matching.len(), "{method}");
        assert_eq!(region_name, matching[0].1);
        assert_eq!(hash, matching[0].2);
    }
}

impl CacheLogger for LoggerMock {
    fn entity_cache_put(&self, n: &str, k: &EntityCacheKey) {
        self.record("entityCachePut", n, &k.hash);
    }
    fn entity_cache_hit(&self, n: &str, k: &EntityCacheKey) {
        self.record("entityCacheHit", n, &k.hash);
    }
    fn entity_cache_miss(&self, n: &str, k: &EntityCacheKey) {
        self.record("entityCacheMiss", n, &k.hash);
    }
    fn collection_cache_put(&self, n: &str, k: &CollectionCacheKey) {
        self.record("collectionCachePut", n, &k.hash);
    }
    fn collection_cache_hit(&self, n: &str, k: &CollectionCacheKey) {
        self.record("collectionCacheHit", n, &k.hash);
    }
    fn collection_cache_miss(&self, n: &str, k: &CollectionCacheKey) {
        self.record("collectionCacheMiss", n, &k.hash);
    }
    fn query_cache_put(&self, n: &str, k: &QueryCacheKey) {
        self.record("queryCachePut", n, &k.hash);
    }
    fn query_cache_hit(&self, n: &str, k: &QueryCacheKey) {
        self.record("queryCacheHit", n, &k.hash);
    }
    fn query_cache_miss(&self, n: &str, k: &QueryCacheKey) {
        self.record("queryCacheMiss", n, &k.hash);
    }
}

fn id1() -> PhpArray {
    std::iter::once(("id", Value::Int(1))).collect()
}

fn set_up() -> (CacheLoggerChain, Arc<LoggerMock>) {
    (CacheLoggerChain::new(), Arc::new(LoggerMock::default()))
}

#[test]
fn testGetAndSetLogger() {
    let (logger, mock) = set_up();
    assert!(logger.get_loggers().is_empty());
    assert!(logger.get_logger("mock").is_none());
    logger.set_logger("mock", mock.clone());
    let got = logger.get_logger("mock").unwrap();
    assert!(std::ptr::addr_eq(Arc::as_ptr(&got), Arc::as_ptr(&mock)));
    let loggers = logger.get_loggers();
    assert_eq!(1, loggers.len());
    assert_eq!("mock", loggers[0].0);
    assert!(std::ptr::addr_eq(
        Arc::as_ptr(&loggers[0].1),
        Arc::as_ptr(&mock)
    ));
}

#[test]
fn testEntityCacheChain() {
    let (logger, mock) = set_up();
    let name = "my_entity_region";
    let key = EntityCacheKey::new(STATE, &id1());
    logger.set_logger("mock", mock.clone());
    logger.entity_cache_hit(name, &key);
    logger.entity_cache_put(name, &key);
    logger.entity_cache_miss(name, &key);
    mock.assert_once("entityCacheHit", name, &key.hash);
    mock.assert_once("entityCachePut", name, &key.hash);
    mock.assert_once("entityCacheMiss", name, &key.hash);
}

#[test]
fn testCollectionCacheChain() {
    let (logger, mock) = set_up();
    let name = "my_collection_region";
    let key = CollectionCacheKey::new(STATE, "cities", &id1());
    logger.set_logger("mock", mock.clone());
    logger.collection_cache_hit(name, &key);
    logger.collection_cache_put(name, &key);
    logger.collection_cache_miss(name, &key);
    mock.assert_once("collectionCacheHit", name, &key.hash);
    mock.assert_once("collectionCachePut", name, &key.hash);
    mock.assert_once("collectionCacheMiss", name, &key.hash);
}

#[test]
fn testQueryCacheChain() {
    let (logger, mock) = set_up();
    let name = "my_query_region";
    let key = QueryCacheKey::new("my_query_hash");
    logger.set_logger("mock", mock.clone());
    logger.query_cache_hit(name, &key);
    logger.query_cache_put(name, &key);
    logger.query_cache_miss(name, &key);
    mock.assert_once("queryCacheHit", name, &key.hash);
    mock.assert_once("queryCachePut", name, &key.hash);
    mock.assert_once("queryCacheMiss", name, &key.hash);
}
