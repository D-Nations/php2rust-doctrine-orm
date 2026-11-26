//! The second-level cache loggers: the `CacheLogger` interface,
//! `CacheLoggerChain`, and `StatisticsCacheLogger`, which counts the
//! hits, misses and puts of each region.
//!
//! Upstream: doctrine/orm src/Cache/Logging/CacheLogger.php
//! Upstream: doctrine/orm src/Cache/Logging/CacheLoggerChain.php
//! Upstream: doctrine/orm src/Cache/Logging/StatisticsCacheLogger.php

use std::fmt::Debug;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::{CollectionCacheKey, EntityCacheKey, QueryCacheKey};

/// `Doctrine\ORM\Cache\Logging\CacheLogger`.
pub trait CacheLogger: Debug + Send + Sync {
    /// Log an entity put into second level cache.
    fn entity_cache_put(&self, region_name: &str, key: &EntityCacheKey);
    /// Log an entity get from second level cache resulted in a hit.
    fn entity_cache_hit(&self, region_name: &str, key: &EntityCacheKey);
    /// Log an entity get from second level cache resulted in a miss.
    fn entity_cache_miss(&self, region_name: &str, key: &EntityCacheKey);
    /// Log an entity put into second level cache.
    fn collection_cache_put(&self, region_name: &str, key: &CollectionCacheKey);
    /// Log an entity get from second level cache resulted in a hit.
    fn collection_cache_hit(&self, region_name: &str, key: &CollectionCacheKey);
    /// Log an entity get from second level cache resulted in a miss.
    fn collection_cache_miss(&self, region_name: &str, key: &CollectionCacheKey);
    /// Log a query put into the query cache.
    fn query_cache_put(&self, region_name: &str, key: &QueryCacheKey);
    /// Log a query get from the query cache resulted in a hit.
    fn query_cache_hit(&self, region_name: &str, key: &QueryCacheKey);
    /// Log a query get from the query cache resulted in a miss.
    fn query_cache_miss(&self, region_name: &str, key: &QueryCacheKey);
}

/// The named loggers of a chain, in the order they were set.
pub type NamedLoggers = Vec<(String, Arc<dyn CacheLogger>)>;

/// `Doctrine\ORM\Cache\Logging\CacheLoggerChain`.
#[derive(Debug, Default)]
pub struct CacheLoggerChain {
    loggers: Mutex<NamedLoggers>,
}

impl CacheLoggerChain {
    pub fn new() -> Self {
        Self::default()
    }

    fn loggers(&self) -> MutexGuard<'_, NamedLoggers> {
        self.loggers.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_logger(&self, name: &str, logger: Arc<dyn CacheLogger>) {
        let mut loggers = self.loggers();
        if let Some(slot) = loggers.iter_mut().find(|(n, _)| n == name) {
            slot.1 = logger;
        } else {
            loggers.push((name.to_owned(), logger));
        }
    }

    pub fn get_logger(&self, name: &str) -> Option<Arc<dyn CacheLogger>> {
        self.loggers()
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, logger)| Arc::clone(logger))
    }

    pub fn get_loggers(&self) -> NamedLoggers {
        self.loggers().clone()
    }

    fn each(&self, f: impl Fn(&dyn CacheLogger)) {
        for (_, logger) in self.get_loggers() {
            f(logger.as_ref());
        }
    }
}

impl CacheLogger for CacheLoggerChain {
    fn entity_cache_put(&self, region_name: &str, key: &EntityCacheKey) {
        self.each(|l| l.entity_cache_put(region_name, key));
    }

    fn entity_cache_hit(&self, region_name: &str, key: &EntityCacheKey) {
        self.each(|l| l.entity_cache_hit(region_name, key));
    }

    fn entity_cache_miss(&self, region_name: &str, key: &EntityCacheKey) {
        self.each(|l| l.entity_cache_miss(region_name, key));
    }

    fn collection_cache_put(&self, region_name: &str, key: &CollectionCacheKey) {
        self.each(|l| l.collection_cache_put(region_name, key));
    }

    fn collection_cache_hit(&self, region_name: &str, key: &CollectionCacheKey) {
        self.each(|l| l.collection_cache_hit(region_name, key));
    }

    fn collection_cache_miss(&self, region_name: &str, key: &CollectionCacheKey) {
        self.each(|l| l.collection_cache_miss(region_name, key));
    }

    fn query_cache_put(&self, region_name: &str, key: &QueryCacheKey) {
        self.each(|l| l.query_cache_put(region_name, key));
    }

    fn query_cache_hit(&self, region_name: &str, key: &QueryCacheKey) {
        self.each(|l| l.query_cache_hit(region_name, key));
    }

    fn query_cache_miss(&self, region_name: &str, key: &QueryCacheKey) {
        self.each(|l| l.query_cache_miss(region_name, key));
    }
}

/// Counts per region name, in the order the regions were first counted.
pub type RegionCounts = Vec<(String, i64)>;

#[derive(Debug, Default)]
struct Counts {
    miss: RegionCounts,
    hit: RegionCounts,
    put: RegionCounts,
}

fn increment(map: &mut RegionCounts, region_name: &str) {
    if let Some(slot) = map.iter_mut().find(|(n, _)| n == region_name) {
        slot.1 += 1;
    } else {
        map.push((region_name.to_owned(), 1));
    }
}

fn set(map: &mut RegionCounts, region_name: &str, value: i64) {
    if let Some(slot) = map.iter_mut().find(|(n, _)| n == region_name) {
        slot.1 = value;
    } else {
        map.push((region_name.to_owned(), value));
    }
}

fn region_count(map: &RegionCounts, region_name: &str) -> i64 {
    map.iter()
        .find(|(n, _)| n == region_name)
        .map_or(0, |(_, count)| *count)
}

/// `Doctrine\ORM\Cache\Logging\StatisticsCacheLogger`: provides basic
/// second level cache statistics.
#[derive(Debug, Default)]
pub struct StatisticsCacheLogger {
    counts: Mutex<Counts>,
}

impl StatisticsCacheLogger {
    pub fn new() -> Self {
        Self::default()
    }

    fn counts(&self) -> MutexGuard<'_, Counts> {
        self.counts.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn miss(&self, region_name: &str) {
        increment(&mut self.counts().miss, region_name);
    }

    fn hit(&self, region_name: &str) {
        increment(&mut self.counts().hit, region_name);
    }

    fn put(&self, region_name: &str) {
        increment(&mut self.counts().put, region_name);
    }

    /// Get the number of entries successfully retrieved from cache.
    pub fn get_region_hit_count(&self, region_name: &str) -> i64 {
        region_count(&self.counts().hit, region_name)
    }

    /// Get the number of cached entries *not* found in cache.
    pub fn get_region_miss_count(&self, region_name: &str) -> i64 {
        region_count(&self.counts().miss, region_name)
    }

    /// Get the number of cacheable entries put in cache.
    pub fn get_region_put_count(&self, region_name: &str) -> i64 {
        region_count(&self.counts().put, region_name)
    }

    pub fn get_regions_miss(&self) -> RegionCounts {
        self.counts().miss.clone()
    }

    pub fn get_regions_hit(&self) -> RegionCounts {
        self.counts().hit.clone()
    }

    pub fn get_regions_put(&self) -> RegionCounts {
        self.counts().put.clone()
    }

    /// Clear region statistics.
    pub fn clear_region_stats(&self, region_name: &str) {
        let mut counts = self.counts();
        set(&mut counts.put, region_name, 0);
        set(&mut counts.hit, region_name, 0);
        set(&mut counts.miss, region_name, 0);
    }

    /// Clear all statistics.
    pub fn clear_stats(&self) {
        *self.counts() = Counts::default();
    }

    /// Get the total number of put in cache.
    pub fn get_put_count(&self) -> i64 {
        self.counts().put.iter().map(|(_, count)| count).sum()
    }

    /// Get the total number of entries successfully retrieved from cache.
    pub fn get_hit_count(&self) -> i64 {
        self.counts().hit.iter().map(|(_, count)| count).sum()
    }

    /// Get the total number of cached entries *not* found in cache.
    pub fn get_miss_count(&self) -> i64 {
        self.counts().miss.iter().map(|(_, count)| count).sum()
    }
}

impl CacheLogger for StatisticsCacheLogger {
    fn entity_cache_put(&self, region_name: &str, _key: &EntityCacheKey) {
        self.put(region_name);
    }

    fn entity_cache_hit(&self, region_name: &str, _key: &EntityCacheKey) {
        self.hit(region_name);
    }

    fn entity_cache_miss(&self, region_name: &str, _key: &EntityCacheKey) {
        self.miss(region_name);
    }

    fn collection_cache_put(&self, region_name: &str, _key: &CollectionCacheKey) {
        self.put(region_name);
    }

    fn collection_cache_hit(&self, region_name: &str, _key: &CollectionCacheKey) {
        self.hit(region_name);
    }

    fn collection_cache_miss(&self, region_name: &str, _key: &CollectionCacheKey) {
        self.miss(region_name);
    }

    fn query_cache_put(&self, region_name: &str, _key: &QueryCacheKey) {
        self.put(region_name);
    }

    fn query_cache_hit(&self, region_name: &str, _key: &QueryCacheKey) {
        self.hit(region_name);
    }

    fn query_cache_miss(&self, region_name: &str, _key: &QueryCacheKey) {
        self.miss(region_name);
    }
}
