//! Upstream: doctrine/orm tests/Tests/ORM/Cache/DefaultRegionTest.php
//!
//! `RegionTestCase`'s tests come from `common/cache.rs`; the pool is
//! the in-memory `ArrayAdapter`. The data provider runs as a loop.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/cache.rs"]
mod cache;

use std::sync::Arc;

use php_value::{Key, Value};
use php2rust_doctrine_orm::cache::pool::{ArrayAdapter, CacheItemPool, PoolValue};
use php2rust_doctrine_orm::cache::{CacheEntry, CollectionCacheEntry, DefaultRegion, Region};

use cache::{array, entry_mock, key_mock};

fn set_up() -> (Arc<dyn CacheItemPool>, DefaultRegion) {
    let pool: Arc<dyn CacheItemPool> = Arc::new(ArrayAdapter::new());
    let region = DefaultRegion::new("default.region.test", Arc::clone(&pool), 0);
    (pool, region)
}

#[test]
fn testPutGetContainsEvict() {
    for (key, value) in cache::data_provider_cache_values() {
        let (_, region) = set_up();
        cache::put_get_contains_evict(&region, &key, &value);
    }
}

#[test]
fn testEvictAll() {
    let (_, region) = set_up();
    cache::evict_all(&region);
}

#[test]
fn testGetters() {
    let (_, region) = set_up();
    assert_eq!("default.region.test", region.get_name());
}

#[test]
fn testSharedRegion() {
    let (pool, _) = set_up();
    let key = key_mock("key");
    let entry = entry_mock(&[("value", Value::string("foo"))]);
    let region1 = DefaultRegion::new("region1", Arc::clone(&pool), 0);
    let region2 = DefaultRegion::new("region2", Arc::clone(&pool), 0);

    assert!(!region1.contains(&key).unwrap());
    assert!(!region2.contains(&key).unwrap());

    region1.put(&key, entry.clone(), None).unwrap();
    region2.put(&key, entry, None).unwrap();

    assert!(region1.contains(&key).unwrap());
    assert!(region2.contains(&key).unwrap());

    region1.evict_all().unwrap();

    assert!(!region1.contains(&key).unwrap());
    assert!(region2.contains(&key).unwrap());
}

#[test]
fn testGetMulti() {
    let (_, region) = set_up();
    let key1 = key_mock("key.1");
    let value1 = entry_mock(&[("id", Value::Int(1)), ("name", Value::string("bar"))]);
    let key2 = key_mock("key.2");
    let value2 = entry_mock(&[("id", Value::Int(2)), ("name", Value::string("bar"))]);

    assert!(!region.contains(&key1).unwrap());
    assert!(!region.contains(&key2).unwrap());

    region.put(&key1, value1.clone(), None).unwrap();
    region.put(&key2, value2.clone(), None).unwrap();

    assert!(region.contains(&key1).unwrap());
    assert!(region.contains(&key2).unwrap());

    let actual = region
        .get_multiple(&CollectionCacheEntry::list([key1, key2]))
        .unwrap()
        .unwrap();
    assert_eq!((Key::from(0), value1), actual[0]);
    assert_eq!((Key::from(1), value2), actual[1]);
}

#[test]
fn testGetMultiPreservesOrderAndKeys() {
    let (_, region) = set_up();
    let key1 = key_mock("key.1");
    let value1 = entry_mock(&[("id", Value::Int(1))]);
    let key2 = key_mock("key.2");
    let value2 = entry_mock(&[("id", Value::Int(2))]);

    region.put(&key1, value1, None).unwrap();
    region.put(&key2, value2, None).unwrap();

    let actual: Vec<_> = region
        .get_multiple(&CollectionCacheEntry::new(vec![
            (Key::from("one"), key1),
            (Key::from("two"), key2),
        ]))
        .unwrap()
        .unwrap()
        .into_iter()
        .map(|(key, entry)| match entry {
            CacheEntry::Other { data, .. } => (key, data),
            other => panic!("{other:?}"),
        })
        .collect();

    assert_eq!(
        vec![
            (Key::from("one"), array(&[("id", Value::Int(1))])),
            (Key::from("two"), array(&[("id", Value::Int(2))])),
        ],
        actual
    );
}

/// `#[Group('GH7266')]`.
#[test]
fn corruptedDataDoesNotLeakIntoApplicationWhenGettingSingleEntry() {
    let (pool, region) = set_up();
    let key1 = key_mock("key.1");
    let item = pool
        .get_item(&format!("DC2_REGION_{}_{}", region.get_name(), key1.hash()))
        .unwrap()
        .set(PoolValue::Value(Value::string("a-very-invalid-value")));
    pool.save(item);

    assert!(region.contains(&key1).unwrap());
    assert!(region.get(&key1).unwrap().is_none());
}

/// `#[Group('GH7266')]`.
#[test]
fn corruptedDataDoesNotLeakIntoApplicationWhenGettingMultipleEntries() {
    let (pool, region) = set_up();
    let key1 = key_mock("key.1");
    let item = pool
        .get_item(&format!("DC2_REGION_{}_{}", region.get_name(), key1.hash()))
        .unwrap()
        .set(PoolValue::Value(Value::string("a-very-invalid-value")));
    pool.save(item);

    assert!(region.contains(&key1).unwrap());
    assert!(
        region
            .get_multiple(&CollectionCacheEntry::list([key1]))
            .unwrap()
            .is_none()
    );
}
