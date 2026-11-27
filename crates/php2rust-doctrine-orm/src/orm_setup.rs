//! Convenience functions to work with the ORM's configuration.
//!
//! `createCacheInstance()` without a cache gives an `ArrayAdapter`: the
//! port has no `APCu`, Memcached or Redis adapter, which upstream tries
//! first outside dev mode (an `ArrayAdapter` is its last choice too), so
//! the cache namespace seed is not used.
//!
//! Upstream: doctrine/orm src/ORMSetup.php

use std::sync::Arc;

use php_value::Thrown;
use php2rust_doctrine_persistence::mapping::driver::FileDriverLocator;

use crate::cache::pool::{ArrayAdapter, CacheItemPool};
use crate::configuration::Configuration;
use crate::mapping::driver::attribute_driver::Paths;
use crate::mapping::driver::xml_driver::DEFAULT_FILE_EXTENSION;
use crate::mapping::driver::{AttributeDriver, XmlDriver};

/// `Doctrine\ORM\ORMSetup`.
pub const CLASS: &str = "Doctrine\\ORM\\ORMSetup";

/// The deprecation of the `create*Configuration()` methods (PHP 8.4).
pub const CONFIGURATION_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12005";

fn deprecated(method: &str, replacement: &str) {
    crate::deprecations::trigger(CONFIGURATION_DEPRECATION, || {
        format!(
            "{CLASS}::{method} is deprecated in favor of {CLASS}::{replacement}(), and will be removed in 4.0."
        )
    });
}

/// `sys_get_temp_dir()`.
fn sys_get_temp_dir() -> String {
    let dir = std::env::temp_dir().to_string_lossy().into_owned();
    match dir.trim_end_matches(['/', '\\']) {
        "" => dir,
        trimmed => trimmed.to_owned(),
    }
}

/// `Doctrine\ORM\ORMSetup`.
pub struct ORMSetup;

impl ORMSetup {
    /// `createAttributeMetadataConfiguration()` (deprecated on PHP 8.4).
    pub fn create_attribute_metadata_configuration(
        paths: impl Into<Paths>,
        is_dev_mode: bool,
        proxy_dir: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
    ) -> Result<Configuration, Thrown> {
        deprecated(
            "createAttributeMetadataConfiguration",
            "createAttributeMetadataConfig",
        );

        let mut config = Self::create_configuration(is_dev_mode, proxy_dir, cache);
        config.set_metadata_driver_impl(Arc::new(AttributeDriver::new(paths, true)?));

        Ok(config)
    }

    /// `createAttributeMetadataConfig()`.
    pub fn create_attribute_metadata_config(
        paths: impl Into<Paths>,
        is_dev_mode: bool,
        cache_namespace_seed: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
    ) -> Result<Configuration, Thrown> {
        let mut config = Self::create_config(is_dev_mode, cache_namespace_seed, cache);
        config.set_metadata_driver_impl(Arc::new(AttributeDriver::new(paths, true)?));

        Ok(config)
    }

    /// `createXMLMetadataConfiguration()` (deprecated on PHP 8.4).
    pub fn create_xml_metadata_configuration(
        paths: Vec<String>,
        is_dev_mode: bool,
        proxy_dir: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
        is_xsd_validation_enabled: bool,
    ) -> Configuration {
        deprecated("createXMLMetadataConfiguration", "createXMLMetadataConfig");

        let mut config = Self::create_configuration(is_dev_mode, proxy_dir, cache);
        config.set_metadata_driver_impl(Arc::new(XmlDriver::create(
            FileDriverLocator::from(paths),
            DEFAULT_FILE_EXTENSION,
            is_xsd_validation_enabled,
        )));

        config
    }

    /// `createXMLMetadataConfig()`.
    pub fn create_xml_metadata_config(
        paths: Vec<String>,
        is_dev_mode: bool,
        cache_namespace_seed: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
        is_xsd_validation_enabled: bool,
    ) -> Configuration {
        let mut config = Self::create_config(is_dev_mode, cache_namespace_seed, cache);
        config.set_metadata_driver_impl(Arc::new(XmlDriver::create(
            FileDriverLocator::from(paths),
            DEFAULT_FILE_EXTENSION,
            is_xsd_validation_enabled,
        )));

        config
    }

    /// `createConfiguration()` (deprecated on PHP 8.4 with a proxy
    /// directory).
    pub fn create_configuration(
        is_dev_mode: bool,
        proxy_dir: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
    ) -> Configuration {
        if proxy_dir.is_some() {
            deprecated("createConfiguration", "createConfig");
        }

        let proxy_dir = proxy_dir
            .filter(|dir| !dir.is_empty() && *dir != "0")
            .map_or_else(sys_get_temp_dir, str::to_owned);

        let cache = Self::create_cache_instance(is_dev_mode, Some(&proxy_dir), cache);

        let mut config = Configuration::new();

        config.set_metadata_cache(Arc::clone(&cache));
        config.set_query_cache(Arc::clone(&cache));
        config.set_result_cache(cache);
        config.set_proxy_dir(proxy_dir);
        config.set_proxy_namespace("DoctrineProxies");
        config.set_auto_generate_proxy_classes(is_dev_mode);

        config
    }

    /// `createConfig()`.
    pub fn create_config(
        is_dev_mode: bool,
        cache_namespace_seed: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
    ) -> Configuration {
        let cache = Self::create_cache_instance(is_dev_mode, cache_namespace_seed, cache);
        let config = Configuration::new();
        config.set_metadata_cache(Arc::clone(&cache));
        config.set_query_cache(Arc::clone(&cache));
        config.set_result_cache(cache);

        config
    }

    fn create_cache_instance(
        _is_dev_mode: bool,
        _cache_namespace_seed: Option<&str>,
        cache: Option<Arc<dyn CacheItemPool>>,
    ) -> Arc<dyn CacheItemPool> {
        if let Some(cache) = cache {
            return cache;
        }

        // In dev mode, and without `APCu`, Memcached or Redis: an ArrayAdapter.
        Arc::new(ArrayAdapter::new())
    }
}
