//! `CacheFactory`, the second-level cache's factory interface, and
//! `DefaultCacheFactory`, which builds the regions over a PSR-6 pool.
//!
//! Upstream: doctrine/orm src/Cache/CacheFactory.php src/Cache/DefaultCacheFactory.php

use std::collections::HashMap;
use std::fmt::Debug;
use std::path::MAIN_SEPARATOR;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value};

use crate::entity_manager::EntityManager;
use crate::mapping::class_metadata::{CACHE_USAGE_NONSTRICT_READ_WRITE, CACHE_USAGE_READ_WRITE};
use crate::mapping::{AssociationMapping, ClassMetadata};
use crate::persisters::EntityPersisterInterface;
use crate::persisters::collection::CollectionPersister;
use crate::utility::to_text;

use super::collection_persister::{AbstractCollectionPersister, CachedCollectionPersister};
use super::default_cache::{Cache, DefaultCache};
use super::exception::{invalid_argument, logic_exception};
use super::hydrator::{
    CollectionHydrator, DefaultCollectionHydrator, DefaultEntityHydrator, EntityHydrator,
};
use super::persister::{AbstractEntityPersister, CacheUsage, CachedEntityPersister};
use super::pool::CacheItemPool;
use super::query_cache::{DefaultQueryCache, QueryCache};
use super::{
    DEFAULT_QUERY_REGION_NAME, DEFAULT_TIMESTAMP_REGION_NAME, DefaultRegion, FileLockRegion,
    Region, RegionsConfiguration, TimestampRegion, UpdateTimestampCache,
};

/// `Doctrine\ORM\Cache\CacheFactory`: contract for building second
/// level cache regions components.
pub trait CacheFactory: Debug + Send + Sync {
    /// Build an entity persister for the given entity metadata.
    fn build_cached_entity_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn EntityPersisterInterface>,
        metadata: Arc<ClassMetadata>,
    ) -> Result<Arc<dyn CachedEntityPersister>, Thrown>;

    /// Build a collection persister for the given relation mapping.
    fn build_cached_collection_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn CollectionPersister>,
        mapping: &AssociationMapping,
    ) -> Result<Arc<dyn CachedCollectionPersister>, Thrown>;

    /// Build a query cache based on the given region name.
    fn build_query_cache(
        &self,
        em: &Arc<EntityManager>,
        region_name: Option<&str>,
    ) -> Result<Arc<dyn QueryCache>, Thrown>;

    /// Build a collection hydrator.
    fn build_collection_hydrator(
        &self,
        em: &Arc<EntityManager>,
        mapping: &AssociationMapping,
    ) -> Arc<dyn CollectionHydrator>;

    /// Build an entity hydrator.
    fn build_entity_hydrator(
        &self,
        em: &Arc<EntityManager>,
        metadata: &Arc<ClassMetadata>,
    ) -> Arc<dyn EntityHydrator>;

    /// Build a cache region from its configuration (`region` and
    /// `usage`).
    fn get_region(&self, cache: &PhpArray) -> Result<Arc<dyn Region>, Thrown>;

    /// Build timestamp cache region.
    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion>;

    /// Build `\Doctrine\ORM\Cache`.
    fn create_cache(&self, em: &Arc<EntityManager>) -> Result<Arc<dyn Cache>, Thrown>;
}

/// `DefaultCacheFactory::buildCachedEntityPersister()` over a factory's
/// `getRegion()` (a factory that replaces `getRegion()` builds its
/// persisters with it).
pub fn build_cached_entity_persister(
    factory: &dyn CacheFactory,
    em: &Arc<EntityManager>,
    persister: Arc<dyn EntityPersisterInterface>,
    metadata: Arc<ClassMetadata>,
) -> Result<Arc<dyn CachedEntityPersister>, Thrown> {
    let cache = metadata.read().cache.clone().unwrap_or_default();
    let region = factory.get_region(&cache)?;
    let usage = cache_field(&cache, "usage");
    let usage_text = to_text(&usage);
    let strategy = match usage {
        Value::Int(usage) => CacheUsage::from_usage(usage),
        _ => None,
    };
    let Some(strategy) = strategy else {
        return Err(invalid_argument(format!(
            "Unrecognized access strategy type [{usage_text}]"
        )));
    };
    if strategy == CacheUsage::ReadWrite && region.as_concurrent().is_none() {
        return Err(invalid_argument(format!(
            "Unable to use access strategy type of [{usage_text}] without a ConcurrentRegion"
        )));
    }
    Ok(Arc::new(AbstractEntityPersister::new(
        strategy, persister, region, em, metadata,
    )?))
}

/// `DefaultCacheFactory::buildCachedCollectionPersister()` over a
/// factory's `getRegion()`.
pub fn build_cached_collection_persister(
    factory: &dyn CacheFactory,
    em: &Arc<EntityManager>,
    persister: Arc<dyn CollectionPersister>,
    mapping: &AssociationMapping,
) -> Result<Arc<dyn CachedCollectionPersister>, Thrown> {
    let cache = mapping.cache.clone().unwrap_or_default();
    let usage = cache_field(&cache, "usage");
    let usage_text = to_text(&usage);
    let region = factory.get_region(&cache)?;
    let strategy = match usage {
        Value::Int(usage) => CacheUsage::from_usage(usage),
        _ => None,
    };
    let Some(strategy) = strategy else {
        return Err(invalid_argument(format!(
            "Unrecognized access strategy type [{usage_text}]"
        )));
    };
    if strategy == CacheUsage::ReadWrite && region.as_concurrent().is_none() {
        return Err(invalid_argument(format!(
            "Unable to use access strategy type of [{usage_text}] without a ConcurrentRegion"
        )));
    }
    Ok(Arc::new(AbstractCollectionPersister::new(
        strategy,
        persister,
        region,
        em,
        mapping.clone(),
    )?))
}

/// `DefaultCacheFactory::buildQueryCache()` over a factory's
/// `getRegion()`.
pub fn build_query_cache(
    factory: &dyn CacheFactory,
    em: &Arc<EntityManager>,
    region_name: Option<&str>,
) -> Result<Arc<dyn QueryCache>, Thrown> {
    let mut cache = PhpArray::new();
    cache.insert(
        "region",
        Value::string(
            region_name
                .filter(|name| !name.is_empty())
                .unwrap_or(DEFAULT_QUERY_REGION_NAME),
        ),
    );
    cache.insert("usage", Value::Int(CACHE_USAGE_NONSTRICT_READ_WRITE));
    Ok(Arc::new(DefaultQueryCache::new(
        em,
        factory.get_region(&cache)?,
    )?))
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
    fn build_cached_entity_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn EntityPersisterInterface>,
        metadata: Arc<ClassMetadata>,
    ) -> Result<Arc<dyn CachedEntityPersister>, Thrown> {
        build_cached_entity_persister(self, em, persister, metadata)
    }

    fn build_cached_collection_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn CollectionPersister>,
        mapping: &AssociationMapping,
    ) -> Result<Arc<dyn CachedCollectionPersister>, Thrown> {
        build_cached_collection_persister(self, em, persister, mapping)
    }

    fn build_query_cache(
        &self,
        em: &Arc<EntityManager>,
        region_name: Option<&str>,
    ) -> Result<Arc<dyn QueryCache>, Thrown> {
        build_query_cache(self, em, region_name)
    }

    fn build_collection_hydrator(
        &self,
        em: &Arc<EntityManager>,
        _mapping: &AssociationMapping,
    ) -> Arc<dyn CollectionHydrator> {
        Arc::new(DefaultCollectionHydrator::new(em))
    }

    fn build_entity_hydrator(
        &self,
        em: &Arc<EntityManager>,
        _metadata: &Arc<ClassMetadata>,
    ) -> Arc<dyn EntityHydrator> {
        Arc::new(DefaultEntityHydrator::new(em))
    }

    fn create_cache(&self, em: &Arc<EntityManager>) -> Result<Arc<dyn Cache>, Thrown> {
        Ok(Arc::new(DefaultCache::new(em)?))
    }

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
