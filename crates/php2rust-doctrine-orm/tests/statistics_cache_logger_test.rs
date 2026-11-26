//! Upstream: doctrine/orm tests/Tests/ORM/Cache/StatisticsCacheLoggerTest.php

#![allow(non_snake_case)]

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::cache::logging::{CacheLogger, StatisticsCacheLogger};
use php2rust_doctrine_orm::cache::{CollectionCacheKey, EntityCacheKey, QueryCacheKey};

const STATE: &str = "Doctrine\\Tests\\Models\\Cache\\State";

fn id1() -> PhpArray {
    std::iter::once(("id", Value::Int(1))).collect()
}

fn assert_counts(logger: &StatisticsCacheLogger, name: &str, total: i64) {
    assert_eq!(total, logger.get_hit_count());
    assert_eq!(total, logger.get_put_count());
    assert_eq!(total, logger.get_miss_count());
    assert_eq!(1, logger.get_region_hit_count(name));
    assert_eq!(1, logger.get_region_put_count(name));
    assert_eq!(1, logger.get_region_miss_count(name));
}

#[test]
fn testEntityCache() {
    let logger = StatisticsCacheLogger::new();
    let name = "my_entity_region";
    let key = EntityCacheKey::new(STATE, &id1());
    logger.entity_cache_hit(name, &key);
    logger.entity_cache_put(name, &key);
    logger.entity_cache_miss(name, &key);
    assert_counts(&logger, name, 1);
}

#[test]
fn testCollectionCache() {
    let logger = StatisticsCacheLogger::new();
    let name = "my_collection_region";
    let key = CollectionCacheKey::new(STATE, "cities", &id1());
    logger.collection_cache_hit(name, &key);
    logger.collection_cache_put(name, &key);
    logger.collection_cache_miss(name, &key);
    assert_counts(&logger, name, 1);
}

#[test]
fn testQueryCache() {
    let logger = StatisticsCacheLogger::new();
    let name = "my_query_region";
    let key = QueryCacheKey::new("my_query_hash");
    logger.query_cache_hit(name, &key);
    logger.query_cache_put(name, &key);
    logger.query_cache_miss(name, &key);
    assert_counts(&logger, name, 1);
}

#[test]
fn testMultipleCaches() {
    let logger = StatisticsCacheLogger::new();
    let cool_region = "my_collection_region";
    let entity_region = "my_entity_region";
    let query_region = "my_query_region";

    let cool_key = CollectionCacheKey::new(STATE, "cities", &id1());
    let entity_key = EntityCacheKey::new(STATE, &id1());
    let query_key = QueryCacheKey::new("my_query_hash");

    logger.query_cache_hit(query_region, &query_key);
    logger.query_cache_put(query_region, &query_key);
    logger.query_cache_miss(query_region, &query_key);

    logger.entity_cache_hit(entity_region, &entity_key);
    logger.entity_cache_put(entity_region, &entity_key);
    logger.entity_cache_miss(entity_region, &entity_key);

    logger.collection_cache_hit(cool_region, &cool_key);
    logger.collection_cache_put(cool_region, &cool_key);
    logger.collection_cache_miss(cool_region, &cool_key);

    assert_counts(&logger, query_region, 3);
    assert_counts(&logger, cool_region, 3);
    assert_counts(&logger, entity_region, 3);

    let miss = logger.get_regions_miss();
    let hit = logger.get_regions_hit();
    let put = logger.get_regions_put();
    for map in [&miss, &put, &hit] {
        for region in [cool_region, query_region, entity_region] {
            assert!(map.iter().any(|(name, _)| name == region), "{region}");
        }
    }
}
