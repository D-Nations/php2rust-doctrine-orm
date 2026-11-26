//! The second-level cache configuration: `CacheConfiguration`,
//! `RegionsConfiguration`, the `QueryCacheValidator` interface and
//! `TimestampQueryCacheValidator`.
//!
//! The configurations are shared (`Arc`) and change in place, as the
//! PHP objects do.
//!
//! Upstream: doctrine/orm src/Cache/CacheConfiguration.php src/Cache/RegionsConfiguration.php
//! Upstream: doctrine/orm src/Cache/QueryCacheValidator.php src/Cache/TimestampQueryCacheValidator.php

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::Thrown;

use super::logging::CacheLogger;
use super::{CacheEntry, CacheFactory, CacheKey, QueryCacheEntry, QueryCacheKey, TimestampRegion};

fn guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `Doctrine\ORM\Cache\QueryCacheValidator`: cache query validator
/// interface.
pub trait QueryCacheValidator: Debug + Send + Sync + std::any::Any {
    /// Checks if the query entry is valid.
    fn is_valid(&self, key: &QueryCacheKey, entry: &QueryCacheEntry) -> Result<bool, Thrown>;
}

/// `Doctrine\ORM\Cache\TimestampQueryCacheValidator`.
#[derive(Debug)]
pub struct TimestampQueryCacheValidator {
    timestamp_region: Arc<dyn TimestampRegion>,
}

impl TimestampQueryCacheValidator {
    pub fn new(timestamp_region: Arc<dyn TimestampRegion>) -> Self {
        Self { timestamp_region }
    }

    fn region_updated(&self, key: &QueryCacheKey, entry: &QueryCacheEntry) -> Result<bool, Thrown> {
        let Some(timestamp_key) = &key.timestamp_key else {
            return Ok(false);
        };
        let timestamp = self
            .timestamp_region
            .get(&CacheKey::Timestamp(timestamp_key.clone()))?;
        Ok(match timestamp {
            Some(CacheEntry::Timestamp(timestamp)) => timestamp.time > entry.time,
            Some(CacheEntry::Query(query)) => query.time > entry.time,
            _ => false,
        })
    }

    /// The timestamp region this validator reads.
    pub fn timestamp_region(&self) -> &Arc<dyn TimestampRegion> {
        &self.timestamp_region
    }
}

impl QueryCacheValidator for TimestampQueryCacheValidator {
    fn is_valid(&self, key: &QueryCacheKey, entry: &QueryCacheEntry) -> Result<bool, Thrown> {
        if self.region_updated(key, entry)? {
            return Ok(false);
        }
        if key.lifetime == 0 {
            return Ok(true);
        }
        #[expect(clippy::cast_precision_loss, reason = "PHP adds an int to a float")]
        let expires = entry.time + key.lifetime as f64;
        Ok(expires > php_value::clock::microtime())
    }
}

#[derive(Debug)]
struct RegionLifetimes {
    default_lifetime: i64,
    default_lock_lifetime: i64,
    by_region: HashMap<String, i64>,
    lock_by_region: HashMap<String, i64>,
}

/// `Doctrine\ORM\Cache\RegionsConfiguration`: the cache regions
/// configuration.
#[derive(Debug)]
pub struct RegionsConfiguration {
    lifetimes: Mutex<RegionLifetimes>,
}

impl Default for RegionsConfiguration {
    fn default() -> Self {
        Self::new(3600, 60)
    }
}

impl RegionsConfiguration {
    /// `new RegionsConfiguration($defaultLifetime, $defaultLockLifetime)`
    /// (3600 and 60 by default).
    pub fn new(default_lifetime: i64, default_lock_lifetime: i64) -> Self {
        Self {
            lifetimes: Mutex::new(RegionLifetimes {
                default_lifetime,
                default_lock_lifetime,
                by_region: HashMap::new(),
                lock_by_region: HashMap::new(),
            }),
        }
    }

    pub fn get_default_lifetime(&self) -> i64 {
        guard(&self.lifetimes).default_lifetime
    }

    pub fn set_default_lifetime(&self, default_lifetime: i64) {
        guard(&self.lifetimes).default_lifetime = default_lifetime;
    }

    pub fn get_default_lock_lifetime(&self) -> i64 {
        guard(&self.lifetimes).default_lock_lifetime
    }

    pub fn set_default_lock_lifetime(&self, default_lock_lifetime: i64) {
        guard(&self.lifetimes).default_lock_lifetime = default_lock_lifetime;
    }

    pub fn get_lifetime(&self, region_name: &str) -> i64 {
        let lifetimes = guard(&self.lifetimes);
        lifetimes
            .by_region
            .get(region_name)
            .copied()
            .unwrap_or(lifetimes.default_lifetime)
    }

    pub fn set_lifetime(&self, name: &str, lifetime: i64) {
        guard(&self.lifetimes)
            .by_region
            .insert(name.to_owned(), lifetime);
    }

    pub fn get_lock_lifetime(&self, region_name: &str) -> i64 {
        let lifetimes = guard(&self.lifetimes);
        lifetimes
            .lock_by_region
            .get(region_name)
            .copied()
            .unwrap_or(lifetimes.default_lock_lifetime)
    }

    pub fn set_lock_lifetime(&self, name: &str, lifetime: i64) {
        guard(&self.lifetimes)
            .lock_by_region
            .insert(name.to_owned(), lifetime);
    }
}

#[derive(Debug, Default)]
struct CacheSettings {
    cache_factory: Option<Arc<dyn CacheFactory>>,
    regions_config: Option<Arc<RegionsConfiguration>>,
    cache_logger: Option<Arc<dyn CacheLogger>>,
    query_validator: Option<Arc<dyn QueryCacheValidator>>,
}

/// `Doctrine\ORM\Cache\CacheConfiguration`: configuration container for
/// second-level cache.
#[derive(Debug, Default)]
pub struct CacheConfiguration {
    settings: Mutex<CacheSettings>,
}

impl CacheConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    fn settings(&self) -> MutexGuard<'_, CacheSettings> {
        guard(&self.settings)
    }

    pub fn get_cache_factory(&self) -> Option<Arc<dyn CacheFactory>> {
        self.settings().cache_factory.clone()
    }

    pub fn set_cache_factory(&self, factory: Arc<dyn CacheFactory>) {
        self.settings().cache_factory = Some(factory);
    }

    pub fn get_cache_logger(&self) -> Option<Arc<dyn CacheLogger>> {
        self.settings().cache_logger.clone()
    }

    pub fn set_cache_logger(&self, logger: Arc<dyn CacheLogger>) {
        self.settings().cache_logger = Some(logger);
    }

    /// `getRegionsConfiguration()`: a default `RegionsConfiguration` the
    /// first time none is set.
    pub fn get_regions_configuration(&self) -> Arc<RegionsConfiguration> {
        Arc::clone(
            self.settings()
                .regions_config
                .get_or_insert_with(|| Arc::new(RegionsConfiguration::default())),
        )
    }

    pub fn set_regions_configuration(&self, regions_config: Arc<RegionsConfiguration>) {
        self.settings().regions_config = Some(regions_config);
    }

    /// `getQueryValidator()`: a `TimestampQueryCacheValidator` over the
    /// cache factory's timestamp region the first time none is set.
    /// Upstream calls `getTimestampRegion()` on a `null` factory, an
    /// `Error`; here that is `None`.
    pub fn get_query_validator(&self) -> Option<Arc<dyn QueryCacheValidator>> {
        let mut settings = self.settings();
        if settings.query_validator.is_none() {
            let factory = settings.cache_factory.clone()?;
            settings.query_validator = Some(Arc::new(TimestampQueryCacheValidator::new(
                factory.get_timestamp_region(),
            )));
        }
        settings.query_validator.clone()
    }

    pub fn set_query_validator(&self, validator: Arc<dyn QueryCacheValidator>) {
        self.settings().query_validator = Some(validator);
    }
}
