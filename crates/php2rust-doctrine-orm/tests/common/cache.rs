//! Helpers shared by the second-level cache tests: the `CacheKeyMock`
//! and `CacheEntryMock` mocks, and `RegionTestCase`'s tests.
//!
//! Upstream: doctrine/orm tests/Tests/Mocks/CacheKeyMock.php tests/Tests/Mocks/CacheEntryMock.php
//! Upstream: doctrine/orm tests/Tests/ORM/Cache/RegionTestCase.php

#![allow(dead_code, reason = "each test file uses a subset")]

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::cache::{CacheEntry, CacheKey, Region};

pub const CACHE_KEY_MOCK: &str = "Doctrine\\Tests\\Mocks\\CacheKeyMock";
pub const CACHE_ENTRY_MOCK: &str = "Doctrine\\Tests\\Mocks\\CacheEntryMock";

/// `new CacheKeyMock($hash)`.
pub fn key_mock(hash: &str) -> CacheKey {
    CacheKey::other(CACHE_KEY_MOCK, hash)
}

/// `new CacheEntryMock($array)`.
pub fn entry_mock(values: &[(&str, Value)]) -> CacheEntry {
    CacheEntry::other(CACHE_ENTRY_MOCK, array(values))
}

/// A PHP array of string keys.
pub fn array(values: &[(&str, Value)]) -> PhpArray {
    values
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect()
}

/// `RegionTestCase::dataProviderCacheValues()`.
pub fn data_provider_cache_values() -> Vec<(CacheKey, CacheEntry)> {
    vec![
        (
            key_mock("key.1"),
            entry_mock(&[("id", Value::Int(1)), ("name", Value::string("bar"))]),
        ),
        (
            key_mock("key.2"),
            entry_mock(&[("id", Value::Int(2)), ("name", Value::string("foo"))]),
        ),
    ]
}

/// `RegionTestCase::testPutGetContainsEvict()` for one data set.
pub fn put_get_contains_evict(region: &dyn Region, key: &CacheKey, value: &CacheEntry) {
    assert!(!region.contains(key).unwrap());
    region.put(key, value.clone(), None).unwrap();
    assert!(region.contains(key).unwrap());
    let actual = region.get(key).unwrap();
    assert_eq!(Some(value), actual.as_ref());
    region.evict(key).unwrap();
    assert!(!region.contains(key).unwrap());
}

/// `RegionTestCase::testEvictAll()`.
pub fn evict_all(region: &dyn Region) {
    let key1 = key_mock("key.1");
    let key2 = key_mock("key.2");
    assert!(!region.contains(&key1).unwrap());
    assert!(!region.contains(&key2).unwrap());
    region
        .put(&key1, entry_mock(&[("value", Value::string("foo"))]), None)
        .unwrap();
    region
        .put(&key2, entry_mock(&[("value", Value::string("bar"))]), None)
        .unwrap();
    assert!(region.contains(&key1).unwrap());
    assert!(region.contains(&key2).unwrap());
    region.evict_all().unwrap();
    assert!(!region.contains(&key1).unwrap());
    assert!(!region.contains(&key2).unwrap());
}
