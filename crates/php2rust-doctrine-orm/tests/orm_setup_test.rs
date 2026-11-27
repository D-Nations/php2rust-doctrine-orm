//! Port of `ORMSetupTest`. The tests run on PHP 8.4, where the
//! `create*Configuration()` methods are deprecated. `NullAdapter` is a
//! pool that keeps nothing (its misses come from an empty
//! `ArrayAdapter`). `assertSame()` of two caches compares the references.
//!
//! `testCacheNamespaceShouldBeGeneratedForApcu*` are `n/a`: they require
//! the `apcu` extension, and the port has no `APCu` adapter.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/ORMSetupTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::Thrown;
use php2rust_doctrine_orm::cache::pool::{ArrayAdapter, CacheItem, CacheItemPool};
use php2rust_doctrine_orm::configuration::Configuration;
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::xml_driver::XmlDriver;
use php2rust_doctrine_orm::orm_setup::{CONFIGURATION_DEPRECATION, ORMSetup};
use php2rust_doctrine_persistence::mapping::driver::FileDriver;

/// `Symfony\Component\Cache\Adapter\NullAdapter`.
#[derive(Debug, Default)]
struct NullAdapter {
    misses: ArrayAdapter,
}

impl CacheItemPool for NullAdapter {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown> {
        self.misses.get_item(key)
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown> {
        self.misses.get_items(keys)
    }

    fn has_item(&self, _key: &str) -> Result<bool, Thrown> {
        Ok(false)
    }

    fn clear(&self, _prefix: &str) -> bool {
        true
    }

    fn delete_item(&self, _key: &str) -> Result<bool, Thrown> {
        Ok(true)
    }

    fn save(&self, _item: CacheItem) -> bool {
        true
    }
}

/// `sys_get_temp_dir()`.
fn sys_get_temp_dir() -> String {
    std::env::temp_dir()
        .to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_owned()
}

/// `expectDeprecationWithIdentifier()`: the deprecation triggers while
/// `run` runs.
fn expect_deprecation<T>(link: &str, run: impl FnOnce() -> T) -> T {
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(link);
    let result = run();
    assert!(
        deprecations::count(link) > before,
        "expected the deprecation {link}"
    );
    result
}

fn is_attribute_driver(config: &Configuration) -> bool {
    config
        .get_metadata_driver_impl()
        .unwrap()
        .downcast_ref::<AttributeDriver>()
        .is_some()
}

fn same(cache: &Arc<dyn CacheItemPool>, other: Option<Arc<dyn CacheItemPool>>) -> bool {
    Arc::ptr_eq(cache, &other.unwrap())
}

#[test]
fn testAttributeConfiguration() {
    let config = expect_deprecation(CONFIGURATION_DEPRECATION, || {
        ORMSetup::create_attribute_metadata_configuration(Vec::<String>::new(), true, None, None)
            .unwrap()
    });

    assert_eq!(Some(sys_get_temp_dir().as_str()), config.get_proxy_dir());
    assert_eq!(Some("DoctrineProxies"), config.get_proxy_namespace());
    assert!(is_attribute_driver(&config));
}

#[test]
fn testAttributeConfig() {
    let config =
        ORMSetup::create_attribute_metadata_config(Vec::<String>::new(), true, None, None).unwrap();

    assert!(is_attribute_driver(&config));
}

#[test]
fn testXMLConfiguration() {
    let config = expect_deprecation(CONFIGURATION_DEPRECATION, || {
        ORMSetup::create_xml_metadata_configuration(Vec::new(), true, None, None, true)
    });

    assert!(
        config
            .get_metadata_driver_impl()
            .unwrap()
            .downcast_ref::<FileDriver<XmlDriver>>()
            .is_some()
    );
}

#[test]
fn testDisablingXmlValidationIsPossible() {
    // expectNotToPerformAssertions()
    ORMSetup::create_xml_metadata_config(
        Vec::new(),
        false,
        None,
        Some(Arc::new(NullAdapter::default())),
        false,
    );
}

#[test]
fn testConfigureProxyDir() {
    let config = ORMSetup::create_attribute_metadata_configuration(
        Vec::<String>::new(),
        true,
        Some("/foo"),
        None,
    )
    .unwrap();
    assert_eq!(Some("/foo"), config.get_proxy_dir());
}

#[test]
fn testConfigureCache() {
    let cache: Arc<dyn CacheItemPool> = Arc::new(ArrayAdapter::new());
    let config = ORMSetup::create_attribute_metadata_config(
        Vec::<String>::new(),
        true,
        None,
        Some(Arc::clone(&cache)),
    )
    .unwrap();

    assert!(same(&cache, config.get_result_cache()));
    assert!(same(&cache, config.get_query_cache()));
    assert!(same(&cache, config.get_metadata_cache()));
}

#[test]
fn testConfigureCacheCustomInstance() {
    let cache: Arc<dyn CacheItemPool> = Arc::new(ArrayAdapter::new());
    let config = ORMSetup::create_config(true, None, Some(Arc::clone(&cache)));

    assert!(same(&cache, config.get_result_cache()));
    assert!(same(&cache, config.get_query_cache()));
    assert!(same(&cache, config.get_metadata_cache()));
}
