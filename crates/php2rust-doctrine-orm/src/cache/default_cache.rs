//! The second-level cache regions API: the `Cache` interface and
//! `DefaultCache`.
//!
//! The collection regions are the regions of the associations' cache
//! configurations (the region a cached collection persister would
//! hold); the identifiers are PHP values.
//!
//! Upstream: doctrine/orm src/Cache.php src/Cache/DefaultCache.php

use std::fmt::Debug;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

use crate::entity_manager::EntityManager;
use crate::exception;
use crate::mapping::ClassMetadata;

use super::hydrator::em_of;
use super::query_cache::QueryCache;
use super::{CacheFactory, CollectionCacheKey, EntityCacheKey, Region};

/// `Doctrine\ORM\Cache`: provides an API for querying/managing the
/// second level cache regions.
pub trait Cache: Debug + Send + Sync {
    /// The entity cache region of an entity class, if it is cached.
    fn get_entity_cache_region(&self, class_name: &str) -> Result<Option<Arc<dyn Region>>, Thrown>;

    /// The collection cache region of an association, if it is cached.
    fn get_collection_cache_region(
        &self,
        class_name: &str,
        association: &str,
    ) -> Result<Option<Arc<dyn Region>>, Thrown>;

    /// Determine whether the cache contains data for the given entity
    /// "instance".
    fn contains_entity(&self, class_name: &str, identifier: &Value) -> Result<bool, Thrown>;

    /// Evicts the entity data for a particular entity "instance".
    fn evict_entity(&self, class_name: &str, identifier: &Value) -> Result<(), Thrown>;

    /// Evicts all entity data from the given region.
    fn evict_entity_region(&self, class_name: &str) -> Result<(), Thrown>;

    /// Evict data from all entity regions.
    fn evict_entity_regions(&self) -> Result<(), Thrown>;

    /// Determine whether the cache contains data for the given collection.
    fn contains_collection(
        &self,
        class_name: &str,
        association: &str,
        owner_identifier: &Value,
    ) -> Result<bool, Thrown>;

    /// Evicts the cache data for the given identified collection instance.
    fn evict_collection(
        &self,
        class_name: &str,
        association: &str,
        owner_identifier: &Value,
    ) -> Result<(), Thrown>;

    /// Evicts all entity data from the given region.
    fn evict_collection_region(&self, class_name: &str, association: &str) -> Result<(), Thrown>;

    /// Evict data from all collection regions.
    fn evict_collection_regions(&self) -> Result<(), Thrown>;

    /// Determine whether the cache contains data for the given query.
    fn contains_query(&self, region_name: &str) -> bool;

    /// Evicts all cached query results under the given name, or default
    /// query cache if the region name is `None`.
    fn evict_query_region(&self, region_name: Option<&str>) -> Result<(), Thrown>;

    /// Evict data from all query regions.
    fn evict_query_regions(&self) -> Result<(), Thrown>;

    /// Get query cache by region name or create a new one if none exist.
    fn get_query_cache(&self, region_name: Option<&str>) -> Result<Arc<dyn QueryCache>, Thrown>;
}

#[derive(Debug, Default)]
struct QueryCaches {
    by_region: Vec<(String, Arc<dyn QueryCache>)>,
    default: Option<Arc<dyn QueryCache>>,
}

/// `Doctrine\ORM\Cache\DefaultCache`.
#[derive(Debug)]
pub struct DefaultCache {
    em: Weak<EntityManager>,
    cache_factory: Arc<dyn CacheFactory>,
    query_caches: Mutex<QueryCaches>,
}

impl DefaultCache {
    pub fn new(em: &Arc<EntityManager>) -> Result<Self, Thrown> {
        let cache_factory = em
            .get_configuration()
            .get_second_level_cache_configuration()
            .ok_or_else(super::persister::no_cache_configuration)?
            .get_cache_factory()
            .ok_or_else(super::persister::no_cache_factory)?;
        Ok(Self {
            em: Arc::downgrade(em),
            cache_factory,
            query_caches: Mutex::default(),
        })
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        em_of(&self.em)
    }

    fn query_caches(&self) -> MutexGuard<'_, QueryCaches> {
        self.query_caches
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn entity_region(
        em: &EntityManager,
        metadata: &ClassMetadata,
    ) -> Result<Option<Arc<dyn Region>>, Thrown> {
        let root = metadata.read().root_entity_name.clone();
        Ok(em
            .get_unit_of_work()
            .get_cached_entity_persister(&root)?
            .map(|persister| persister.get_cache_region()))
    }

    fn collection_region(
        em: &EntityManager,
        metadata: &ClassMetadata,
        association: &str,
    ) -> Result<Option<Arc<dyn Region>>, Thrown> {
        let mapping = metadata
            .read()
            .get_association_mapping(association)?
            .clone();
        em.get_unit_of_work().get_collection_cache_region(&mapping)
    }

    fn build_entity_cache_key(
        em: &EntityManager,
        metadata: &ClassMetadata,
        identifier: &Value,
    ) -> Result<EntityCacheKey, Thrown> {
        let identifier = match identifier {
            Value::Array(identifier) => identifier.clone(),
            other => Self::to_identifier_array(em, metadata, other)?,
        };
        Ok(EntityCacheKey::new(
            metadata.read().root_entity_name.clone(),
            &identifier,
        ))
    }

    fn build_collection_cache_key(
        em: &EntityManager,
        metadata: &ClassMetadata,
        association: &str,
        owner_identifier: &Value,
    ) -> Result<CollectionCacheKey, Thrown> {
        let owner_identifier = match owner_identifier {
            Value::Array(identifier) => identifier.clone(),
            other => Self::to_identifier_array(em, metadata, other)?,
        };
        Ok(CollectionCacheKey::new(
            metadata.read().root_entity_name.clone(),
            association,
            &owner_identifier,
        ))
    }

    /// `toIdentifierArray()` (private upstream; its test calls it by
    /// reflection).
    pub fn to_identifier_array(
        em: &EntityManager,
        metadata: &ClassMetadata,
        identifier: &Value,
    ) -> Result<PhpArray, Thrown> {
        let mut identifier = identifier.clone();
        if let Value::Object(object) = &identifier {
            let class = object.class().to_owned();
            if em.get_metadata_factory().has_metadata_for(&class) {
                identifier = single_identifier(em, object)?
                    .ok_or_else(|| exception::invalid_identifier_binding_entity(&class))?;
            }
        }
        let mut id = PhpArray::new();
        id.insert(
            metadata
                .read()
                .identifier
                .first()
                .cloned()
                .unwrap_or_default()
                .as_str(),
            identifier,
        );
        Ok(id)
    }
}

/// `$uow->getSingleIdentifierValue($entity)`, `None` for `null`.
fn single_identifier(em: &EntityManager, entity: &Object) -> Result<Option<Value>, Thrown> {
    let value = em.get_unit_of_work().get_single_identifier_value(entity)?;
    Ok((!value.is_null()).then_some(value))
}

impl Cache for DefaultCache {
    fn get_entity_cache_region(&self, class_name: &str) -> Result<Option<Arc<dyn Region>>, Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        Self::entity_region(&em, &metadata)
    }

    fn get_collection_cache_region(
        &self,
        class_name: &str,
        association: &str,
    ) -> Result<Option<Arc<dyn Region>>, Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        Self::collection_region(&em, &metadata, association)
    }

    fn contains_entity(&self, class_name: &str, identifier: &Value) -> Result<bool, Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        let Some(region) = Self::entity_region(&em, &metadata)? else {
            return Ok(false);
        };
        region.contains(&Self::build_entity_cache_key(&em, &metadata, identifier)?.into())
    }

    fn evict_entity(&self, class_name: &str, identifier: &Value) -> Result<(), Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        let Some(region) = Self::entity_region(&em, &metadata)? else {
            return Ok(());
        };
        region.evict(&Self::build_entity_cache_key(&em, &metadata, identifier)?.into())?;
        Ok(())
    }

    fn evict_entity_region(&self, class_name: &str) -> Result<(), Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        if let Some(region) = Self::entity_region(&em, &metadata)? {
            region.evict_all()?;
        }
        Ok(())
    }

    fn evict_entity_regions(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        for metadata in em.get_metadata_factory().get_all_metadata()? {
            if let Some(region) = Self::entity_region(&em, &metadata)? {
                region.evict_all()?;
            }
        }
        Ok(())
    }

    fn contains_collection(
        &self,
        class_name: &str,
        association: &str,
        owner_identifier: &Value,
    ) -> Result<bool, Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        let Some(region) = Self::collection_region(&em, &metadata, association)? else {
            return Ok(false);
        };
        region.contains(
            &Self::build_collection_cache_key(&em, &metadata, association, owner_identifier)?
                .into(),
        )
    }

    fn evict_collection(
        &self,
        class_name: &str,
        association: &str,
        owner_identifier: &Value,
    ) -> Result<(), Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        let Some(region) = Self::collection_region(&em, &metadata, association)? else {
            return Ok(());
        };
        region.evict(
            &Self::build_collection_cache_key(&em, &metadata, association, owner_identifier)?
                .into(),
        )?;
        Ok(())
    }

    fn evict_collection_region(&self, class_name: &str, association: &str) -> Result<(), Thrown> {
        let em = self.em()?;
        let metadata = em.get_class_metadata(class_name)?;
        if let Some(region) = Self::collection_region(&em, &metadata, association)? {
            region.evict_all()?;
        }
        Ok(())
    }

    fn evict_collection_regions(&self) -> Result<(), Thrown> {
        let em = self.em()?;
        for metadata in em.get_metadata_factory().get_all_metadata()? {
            let associations: Vec<_> = metadata
                .read()
                .association_mappings
                .values()
                .filter(|association| association.is_to_many())
                .cloned()
                .collect();
            for association in associations {
                if let Some(region) = em
                    .get_unit_of_work()
                    .get_collection_cache_region(&association)?
                {
                    region.evict_all()?;
                }
            }
        }
        Ok(())
    }

    fn contains_query(&self, region_name: &str) -> bool {
        self.query_caches()
            .by_region
            .iter()
            .any(|(name, _)| name == region_name)
    }

    fn evict_query_region(&self, region_name: Option<&str>) -> Result<(), Thrown> {
        let query_cache = {
            let caches = self.query_caches();
            match region_name {
                None if caches.default.is_some() => caches.default.clone(),
                // `$this->queryCaches[null]`: no region is named so.
                None => None,
                Some(name) => caches
                    .by_region
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, cache)| Arc::clone(cache)),
            }
        };
        if let Some(query_cache) = query_cache {
            query_cache.clear()?;
        }
        Ok(())
    }

    fn evict_query_regions(&self) -> Result<(), Thrown> {
        self.get_query_cache(None)?.clear()?;
        let caches: Vec<_> = self
            .query_caches()
            .by_region
            .iter()
            .map(|(_, cache)| Arc::clone(cache))
            .collect();
        for query_cache in caches {
            query_cache.clear()?;
        }
        Ok(())
    }

    fn get_query_cache(&self, region_name: Option<&str>) -> Result<Arc<dyn QueryCache>, Thrown> {
        let Some(region_name) = region_name else {
            if let Some(cache) = &self.query_caches().default {
                return Ok(Arc::clone(cache));
            }
            let cache = self.cache_factory.build_query_cache(&self.em()?, None)?;
            self.query_caches().default = Some(Arc::clone(&cache));
            return Ok(cache);
        };
        if let Some((_, cache)) = self
            .query_caches()
            .by_region
            .iter()
            .find(|(name, _)| name == region_name)
        {
            return Ok(Arc::clone(cache));
        }
        let cache = self
            .cache_factory
            .build_query_cache(&self.em()?, Some(region_name))?;
        self.query_caches()
            .by_region
            .push((region_name.to_owned(), Arc::clone(&cache)));
        Ok(cache)
    }
}
