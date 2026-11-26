//! `CacheFactory`, the second-level cache's factory interface, and
//! `DefaultCacheFactory`, which builds the regions over a PSR-6 pool.
//!
//! Upstream: doctrine/orm src/Cache/CacheFactory.php src/Cache/DefaultCacheFactory.php

use std::collections::HashMap;
use std::fmt::Debug;
use std::path::MAIN_SEPARATOR;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value};

use crate::mapping::class_metadata::CACHE_USAGE_READ_WRITE;
use crate::utility::to_text;

use super::exception::logic_exception;
use super::pool::CacheItemPool;
use super::{
    DEFAULT_TIMESTAMP_REGION_NAME, DefaultRegion, FileLockRegion, Region, RegionsConfiguration,
    TimestampRegion, UpdateTimestampCache,
};

/// `Doctrine\ORM\Cache\CacheFactory`: contract for building second
/// level cache regions components.
pub trait CacheFactory: Debug + Send + Sync {
    /// Build a cache region from its configuration (`region` and
    /// `usage`).
    fn get_region(&self, cache: &PhpArray) -> Result<Arc<dyn Region>, Thrown>;

    /// Build timestamp cache region.
    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion>;
}

#[derive(Debug, Default)]
struct FactoryState {
    timestamp_region: Option<Arc<dyn TimestampRegion>>,
    regions: HashMap<String, Arc<dyn Region>>,
    file_lock_region_directory: Option<String>,
}

/// `Doctrine\ORM\Cache\DefaultCacheFactory`.
#[derive(Debug)]
pub struct DefaultCacheFactory {
    regions_config: Arc<RegionsConfiguration>,
    cache_item_pool: Arc<dyn CacheItemPool>,
    state: Mutex<FactoryState>,
}

impl DefaultCacheFactory {
    pub fn new(
        regions_config: Arc<RegionsConfiguration>,
        cache_item_pool: Arc<dyn CacheItemPool>,
    ) -> Self {
        Self {
            regions_config,
            cache_item_pool,
            state: Mutex::default(),
        }
    }

    fn state(&self) -> MutexGuard<'_, FactoryState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn set_file_lock_region_directory(&self, file_lock_region_directory: &str) {
        self.state().file_lock_region_directory = Some(file_lock_region_directory.to_owned());
    }

    pub fn get_file_lock_region_directory(&self) -> Option<String> {
        self.state().file_lock_region_directory.clone()
    }

    pub fn set_region(&self, region: Arc<dyn Region>) {
        self.state().regions.insert(region.get_name(), region);
    }

    pub fn set_timestamp_region(&self, region: Arc<dyn TimestampRegion>) {
        self.state().timestamp_region = Some(region);
    }

    /// The pool the regions are built over.
    pub fn cache_item_pool(&self) -> &Arc<dyn CacheItemPool> {
        &self.cache_item_pool
    }
}

/// `$cache[$name]` of a region configuration.
pub(crate) fn cache_field(cache: &PhpArray, name: &str) -> Value {
    cache.get(&Key::from(name)).cloned().unwrap_or(Value::Null)
}

impl CacheFactory for DefaultCacheFactory {
    fn get_region(&self, cache: &PhpArray) -> Result<Arc<dyn Region>, Thrown> {
        let name = to_text(&cache_field(cache, "region"));
        if let Some(region) = self.state().regions.get(&name) {
            return Ok(Arc::clone(region));
        }
        let lifetime = self.regions_config.get_lifetime(&name);
        let mut region: Arc<dyn Region> = Arc::new(DefaultRegion::new(
            name.as_str(),
            Arc::clone(&self.cache_item_pool),
            lifetime,
        ));
        if cache_field(cache, "usage") == Value::Int(CACHE_USAGE_READ_WRITE) {
            let directory = self
                .get_file_lock_region_directory()
                .filter(|directory| !directory.is_empty());
            let Some(directory) = directory else {
                return Err(logic_exception(
                    "If you want to use a \"READ_WRITE\" cache an implementation of \"Doctrine\\ORM\\Cache\\ConcurrentRegion\" is required, The default implementation provided by doctrine is \"Doctrine\\ORM\\Cache\\Region\\FileLockRegion\" if you want to use it please provide a valid directory, DefaultCacheFactory#setFileLockRegionDirectory(). ".to_owned(),
                ));
            };
            let directory = format!("{directory}{MAIN_SEPARATOR}{name}");
            region = Arc::new(FileLockRegion::new(
                region,
                directory,
                self.regions_config.get_lock_lifetime(&name),
            )?);
        }
        self.state().regions.insert(name, Arc::clone(&region));
        Ok(region)
    }

    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion> {
        let mut state = self.state();
        Arc::clone(state.timestamp_region.get_or_insert_with(|| {
            let lifetime = self
                .regions_config
                .get_lifetime(DEFAULT_TIMESTAMP_REGION_NAME);
            Arc::new(UpdateTimestampCache::new(
                DEFAULT_TIMESTAMP_REGION_NAME,
                Arc::clone(&self.cache_item_pool),
                lifetime,
            ))
        }))
    }
}
