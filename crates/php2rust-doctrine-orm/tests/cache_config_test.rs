//! Upstream: doctrine/orm tests/Tests/ORM/Cache/CacheConfigTest.php
//!
//! The stubs (`createStub()`) are small implementations of the
//! interfaces; `assertEquals()` of a stub compares the `Arc`s, and
//! `assertInstanceOf()` downcasts.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::any::Any;
use std::sync::Arc;

use php_value::{PhpArray, Thrown};
use php2rust_doctrine_orm::cache::logging::CacheLogger;
use php2rust_doctrine_orm::cache::region::MultipleEntries;
use php2rust_doctrine_orm::cache::{
    CacheConfiguration, CacheEntry, CacheFactory, CacheKey, CollectionCacheEntry,
    CollectionCacheKey, EntityCacheKey, Lock, QueryCacheEntry, QueryCacheKey, QueryCacheValidator,
    Region, TimestampQueryCacheValidator, TimestampRegion,
};

/// `assertSame()` of two `Arc`s of a trait object.
fn same<T: ?Sized, U: ?Sized>(a: &Arc<T>, b: &Arc<U>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

#[derive(Debug)]
struct LoggerStub;

impl CacheLogger for LoggerStub {
    fn entity_cache_put(&self, _: &str, _: &EntityCacheKey) {}
    fn entity_cache_hit(&self, _: &str, _: &EntityCacheKey) {}
    fn entity_cache_miss(&self, _: &str, _: &EntityCacheKey) {}
    fn collection_cache_put(&self, _: &str, _: &CollectionCacheKey) {}
    fn collection_cache_hit(&self, _: &str, _: &CollectionCacheKey) {}
    fn collection_cache_miss(&self, _: &str, _: &CollectionCacheKey) {}
    fn query_cache_put(&self, _: &str, _: &QueryCacheKey) {}
    fn query_cache_hit(&self, _: &str, _: &QueryCacheKey) {}
    fn query_cache_miss(&self, _: &str, _: &QueryCacheKey) {}
}

/// `createStub(TimestampRegion::class)`: every method returns its type's
/// default.
#[derive(Debug)]
struct TimestampRegionStub;

impl Region for TimestampRegionStub {
    fn get_name(&self) -> String {
        String::new()
    }
    fn contains(&self, _: &CacheKey) -> Result<bool, Thrown> {
        Ok(false)
    }
    fn get(&self, _: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        Ok(None)
    }
    fn get_multiple(&self, _: &CollectionCacheEntry) -> Result<Option<MultipleEntries>, Thrown> {
        Ok(None)
    }
    fn put(&self, _: &CacheKey, _: CacheEntry, _: Option<&Lock>) -> Result<bool, Thrown> {
        Ok(false)
    }
    fn evict(&self, _: &CacheKey) -> Result<bool, Thrown> {
        Ok(false)
    }
    fn evict_all(&self) -> Result<bool, Thrown> {
        Ok(false)
    }
    fn as_timestamp(&self) -> Option<&dyn TimestampRegion> {
        Some(self)
    }
}

impl TimestampRegion for TimestampRegionStub {
    fn update(&self, _: &CacheKey) -> Result<(), Thrown> {
        Ok(())
    }
}

/// `createStub(CacheFactory::class)`, with `getTimestampRegion()`
/// returning a timestamp region stub.
#[derive(Debug)]
struct FactoryStub;

impl CacheFactory for FactoryStub {
    fn get_region(&self, _: &PhpArray) -> Result<Arc<dyn Region>, Thrown> {
        Ok(Arc::new(TimestampRegionStub))
    }
    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion> {
        Arc::new(TimestampRegionStub)
    }
}

#[derive(Debug)]
struct ValidatorStub;

impl QueryCacheValidator for ValidatorStub {
    fn is_valid(&self, _: &QueryCacheKey, _: &QueryCacheEntry) -> Result<bool, Thrown> {
        Ok(false)
    }
}

fn set_up() -> CacheConfiguration {
    CacheConfiguration::new()
}

#[test]
fn testSetGetRegionLifetime() {
    let config = set_up().get_regions_configuration();
    config.set_default_lifetime(111);
    assert_eq!(
        config.get_default_lifetime(),
        config.get_lifetime("foo_region")
    );
    config.set_lifetime("foo_region", 222);
    assert_eq!(222, config.get_lifetime("foo_region"));
}

#[test]
fn testSetGetCacheLogger() {
    let config = set_up();
    let logger: Arc<dyn CacheLogger> = Arc::new(LoggerStub);
    assert!(config.get_cache_logger().is_none());
    config.set_cache_logger(Arc::clone(&logger));
    assert!(same(&logger, &config.get_cache_logger().unwrap()));
}

#[test]
fn testSetGetCacheFactory() {
    let config = set_up();
    let factory: Arc<dyn CacheFactory> = Arc::new(FactoryStub);
    assert!(config.get_cache_factory().is_none());
    config.set_cache_factory(Arc::clone(&factory));
    assert!(same(&factory, &config.get_cache_factory().unwrap()));
}

#[test]
fn testSetGetQueryValidator() {
    let config = set_up();
    config.set_cache_factory(Arc::new(FactoryStub));
    let validator: Arc<dyn QueryCacheValidator> = Arc::new(ValidatorStub);
    let default = config.get_query_validator().unwrap();
    assert!(
        (default.as_ref() as &dyn Any)
            .downcast_ref::<TimestampQueryCacheValidator>()
            .is_some()
    );
    config.set_query_validator(Arc::clone(&validator));
    assert!(same(&validator, &config.get_query_validator().unwrap()));
}
