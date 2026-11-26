//! The second-level cache's collection persisters: the
//! `CachedCollectionPersister` interface, and `AbstractCollectionPersister`,
//! which wraps a collection persister with a region under one of the
//! three access strategies (`ReadOnlyCachedCollectionPersister`,
//! `NonStrictReadWriteCachedCollectionPersister`,
//! `ReadWriteCachedCollectionPersister`; [`CacheUsage`] names which, and
//! `get_class_name()` gives the upstream class).
//!
//! A collection's elements are a PHP array of entities. `$queuedCache`
//! keeps its items by the collection's object id, as `spl_object_id()`
//! does; `count($queuedCache)` is `queued_cache_len()`.
//!
//! Upstream: doctrine/orm src/Cache/Persister/Collection/CachedCollectionPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Collection/AbstractCollectionPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Collection/ReadOnlyCachedCollectionPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Collection/NonStrictReadWriteCachedCollectionPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Collection/ReadWriteCachedCollectionPersister.php

use std::fmt::Debug;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::{Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;

use crate::entity_manager::EntityManager;
use crate::mapping::{AssociationMapping, ClassMetadata};
use crate::persistent_collection::PersistentCollection;
use crate::persisters::collection::CollectionPersister;
use crate::unit_of_work::Fut;

use super::exception::cannot_update_read_only_collection;
use super::hydrator::{CollectionHydrator, em_of};
use super::logging::CacheLogger;
use super::persister::{
    CacheUsage, CachedPersister, no_cache_configuration, no_cache_factory, not_cached,
};
use super::{CacheEntry, CollectionCacheKey, Lock, Region};

/// `Doctrine\ORM\Cache\Persister\Collection\CachedCollectionPersister`:
/// interface for second level cache collection persisters.
pub trait CachedCollectionPersister: CachedPersister + CollectionPersister {
    fn get_source_entity_metadata(&self) -> Arc<ClassMetadata>;

    fn get_target_entity_metadata(&self) -> Arc<ClassMetadata>;

    /// Loads a collection from cache.
    fn load_collection_cache<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        key: &'a CollectionCacheKey,
    ) -> Fut<'a, Option<PhpArray>>;

    /// Stores a collection into cache (`$elements` as an array).
    fn store_collection_cache(
        &self,
        key: &CollectionCacheKey,
        elements: &PhpArray,
    ) -> Result<(), Thrown>;

    /// The upstream class of this persister.
    fn get_class_name(&self) -> &'static str;
}

pub const READ_ONLY_CACHED_COLLECTION_PERSISTER: &str =
    "Doctrine\\ORM\\Cache\\Persister\\Collection\\ReadOnlyCachedCollectionPersister";
pub const NON_STRICT_READ_WRITE_CACHED_COLLECTION_PERSISTER: &str =
    "Doctrine\\ORM\\Cache\\Persister\\Collection\\NonStrictReadWriteCachedCollectionPersister";
pub const READ_WRITE_CACHED_COLLECTION_PERSISTER: &str =
    "Doctrine\\ORM\\Cache\\Persister\\Collection\\ReadWriteCachedCollectionPersister";

/// A queued `update` or `delete` of `$queuedCache`.
#[derive(Debug, Clone)]
enum Queued {
    /// `['key' => $key, 'list' => $collection]` (non-strict read-write
    /// update).
    List {
        key: CollectionCacheKey,
        list: Arc<PersistentCollection>,
    },
    /// The key (non-strict read-write delete).
    Key(CollectionCacheKey),
    /// `['key' => $key, 'lock' => $lock]` (read-write).
    Locked {
        key: CollectionCacheKey,
        #[allow(dead_code, reason = "the lock upstream keeps with the key")]
        lock: Lock,
    },
}

impl Queued {
    fn key(&self) -> &CollectionCacheKey {
        match self {
            Self::List { key, .. } | Self::Key(key) | Self::Locked { key, .. } => key,
        }
    }
}

/// `$queuedCache[$name][spl_object_id($collection)]`, in insertion order.
type QueuedItems = Vec<(usize, Queued)>;

#[derive(Debug, Default)]
struct QueuedCache {
    update: Option<QueuedItems>,
    delete: Option<QueuedItems>,
}

fn queue(items: &mut Option<QueuedItems>, id: usize, item: Queued) {
    let items = items.get_or_insert_with(Vec::new);
    if let Some(slot) = items.iter_mut().find(|(oid, _)| *oid == id) {
        slot.1 = item;
    } else {
        items.push((id, item));
    }
}

fn object_id(collection: &Arc<PersistentCollection>) -> usize {
    Arc::as_ptr(collection) as usize
}

/// `Doctrine\ORM\Cache\Persister\Collection\AbstractCollectionPersister`
/// and its three access strategies.
pub struct AbstractCollectionPersister {
    usage: CacheUsage,
    persister: Arc<dyn CollectionPersister>,
    region: Arc<dyn Region>,
    em: Weak<EntityManager>,
    association: AssociationMapping,
    source_entity: Arc<ClassMetadata>,
    target_entity: Arc<ClassMetadata>,
    queued_cache: Mutex<QueuedCache>,
    region_name: String,
    hydrator: Arc<dyn CollectionHydrator>,
    cache_logger: Option<Arc<dyn CacheLogger>>,
}

impl Debug for AbstractCollectionPersister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(CachedCollectionPersister::get_class_name(self))
            .field("region_name", &self.region_name)
            .finish_non_exhaustive()
    }
}

impl AbstractCollectionPersister {
    /// `new ReadOnlyCachedCollectionPersister(...)`,
    /// `new NonStrictReadWriteCachedCollectionPersister(...)` or
    /// `new ReadWriteCachedCollectionPersister(...)` (whose region is a
    /// `ConcurrentRegion`).
    pub fn new(
        usage: CacheUsage,
        persister: Arc<dyn CollectionPersister>,
        region: Arc<dyn Region>,
        em: &Arc<EntityManager>,
        association: AssociationMapping,
    ) -> Result<Self, Thrown> {
        if usage == CacheUsage::ReadWrite && region.as_concurrent().is_none() {
            return Err(Thrown::new(
                "TypeError",
                "Doctrine\\ORM\\Cache\\Persister\\Collection\\ReadWriteCachedCollectionPersister::__construct(): Argument #2 ($region) must be of type Doctrine\\ORM\\Cache\\ConcurrentRegion",
                0,
            ));
        }
        let configuration = em.get_configuration();
        let cache_config = configuration
            .get_second_level_cache_configuration()
            .ok_or_else(no_cache_configuration)?;
        let cache_factory = cache_config
            .get_cache_factory()
            .ok_or_else(no_cache_factory)?;
        // `$this->filters = $em->getFilters()`.
        let _ = em.get_filters();
        let hydrator = cache_factory.build_collection_hydrator(em, &association);
        let source_entity = em.get_class_metadata(&association.source_entity)?;
        let target_entity = em.get_class_metadata(&association.target_entity)?;
        Ok(Self {
            usage,
            region_name: region.get_name(),
            persister,
            region,
            em: Arc::downgrade(em),
            association,
            source_entity,
            target_entity,
            queued_cache: Mutex::default(),
            hydrator,
            cache_logger: cache_config.get_cache_logger(),
        })
    }

    /// The access strategy.
    pub fn usage(&self) -> CacheUsage {
        self.usage
    }

    /// `count($this->queuedCache)`.
    pub fn queued_cache_len(&self) -> usize {
        let queued = self.queued();
        usize::from(queued.update.is_some()) + usize::from(queued.delete.is_some())
    }

    fn queued(&self) -> MutexGuard<'_, QueuedCache> {
        self.queued_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        em_of(&self.em)
    }

    /// `new CollectionCacheKey($this->sourceEntity->rootEntityName,
    /// $this->association->fieldName, $ownerId, $this->filters->getHash())`.
    fn collection_key(
        &self,
        collection: &PersistentCollection,
    ) -> Result<CollectionCacheKey, Thrown> {
        let em = self.em()?;
        let owner = collection.get_owner().ok_or_else(|| {
            Thrown::new(
                "TypeError",
                "Doctrine\\ORM\\UnitOfWork::getEntityIdentifier(): Argument #1 ($entity) must be of type object, null given",
                0,
            )
        })?;
        let owner_id = em.get_unit_of_work().get_entity_identifier(&owner)?;
        let root = self.source_entity.read().root_entity_name.clone();
        Ok(CollectionCacheKey::with_filter_hash(
            root,
            self.association.field_name.as_str(),
            &owner_id,
            &em.get_filters().get_hash(),
        ))
    }

    fn concurrent_lock(&self, key: &CollectionCacheKey) -> Result<Option<Lock>, Thrown> {
        match self.region.as_concurrent() {
            Some(region) => region.lock(&key.clone().into()),
            None => Ok(None),
        }
    }

    fn evict_queued(&self, items: Option<&QueuedItems>) -> Result<(), Thrown> {
        for (_, item) in items.into_iter().flatten() {
            self.region.evict(&item.key().clone().into())?;
        }
        Ok(())
    }

    async fn do_delete(&self, collection: &Arc<PersistentCollection>) -> Result<(), Thrown> {
        let key = self.collection_key(collection)?;
        if self.usage == CacheUsage::ReadWrite {
            let lock = self.concurrent_lock(&key)?;
            self.persister.delete(collection).await?;
            let Some(lock) = lock else {
                return Ok(());
            };
            queue(
                &mut self.queued().delete,
                object_id(collection),
                Queued::Locked { key, lock },
            );
            return Ok(());
        }
        self.persister.delete(collection).await?;
        queue(
            &mut self.queued().delete,
            object_id(collection),
            Queued::Key(key),
        );
        Ok(())
    }

    async fn do_update(&self, collection: &Arc<PersistentCollection>) -> Result<(), Thrown> {
        if self.usage == CacheUsage::ReadOnly
            && collection.is_dirty()
            && !collection.get_snapshot().is_empty()
        {
            let owner = collection.get_owner();
            return Err(cannot_update_read_only_collection(
                owner.as_ref().map_or("", |owner| owner.class()),
                &self.association.field_name,
            ));
        }

        let is_initialized = collection.is_initialized();
        let is_dirty = collection.is_dirty();

        if !is_initialized && !is_dirty {
            return Ok(());
        }

        if self.usage == CacheUsage::ReadWrite {
            self.persister.update(collection).await?;
            let key = self.collection_key(collection)?;
            let Some(lock) = self.concurrent_lock(&key)? else {
                return Ok(());
            };
            queue(
                &mut self.queued().update,
                object_id(collection),
                Queued::Locked { key, lock },
            );
            return Ok(());
        }

        let key = self.collection_key(collection)?;

        // Invalidate non initialized collections OR ordered collection
        if (is_dirty && !is_initialized) || self.association.is_ordered() {
            self.persister.update(collection).await?;
            queue(
                &mut self.queued().delete,
                object_id(collection),
                Queued::Key(key),
            );
            return Ok(());
        }

        self.persister.update(collection).await?;
        queue(
            &mut self.queued().update,
            object_id(collection),
            Queued::List {
                key,
                list: Arc::clone(collection),
            },
        );
        Ok(())
    }

    async fn do_count(&self, collection: &Arc<PersistentCollection>) -> Result<i64, Thrown> {
        let key = self.collection_key(collection)?;
        if let Some(entry) = self.region.get(&key.into())? {
            let identifiers = match &entry {
                CacheEntry::Collection(entry) => entry.identifiers.len(),
                _ => {
                    return Err(Thrown::new("Error", "Undefined property: $identifiers", 0));
                }
            };
            return Ok(i64::try_from(identifiers).unwrap_or(i64::MAX));
        }
        self.persister.count(collection).await
    }
}

impl CachedPersister for AbstractCollectionPersister {
    fn after_transaction_complete(&self) -> Result<(), Thrown> {
        let queued = std::mem::take(&mut *self.queued());
        if self.usage == CacheUsage::ReadWrite {
            self.evict_queued(queued.update.as_ref())?;
            self.evict_queued(queued.delete.as_ref())?;
            return Ok(());
        }
        for (_, item) in queued.update.iter().flatten() {
            if let Queued::List { key, list } = item {
                self.store_collection_cache(key, &list.unwrap())?;
            }
        }
        self.evict_queued(queued.delete.as_ref())?;
        Ok(())
    }

    fn after_transaction_rolled_back(&self) -> Result<(), Thrown> {
        let queued = std::mem::take(&mut *self.queued());
        if self.usage == CacheUsage::ReadWrite {
            self.evict_queued(queued.update.as_ref())?;
            self.evict_queued(queued.delete.as_ref())?;
        }
        Ok(())
    }

    fn get_cache_region(&self) -> Arc<dyn Region> {
        Arc::clone(&self.region)
    }
}

impl CachedCollectionPersister for AbstractCollectionPersister {
    fn get_source_entity_metadata(&self) -> Arc<ClassMetadata> {
        Arc::clone(&self.source_entity)
    }

    fn get_target_entity_metadata(&self) -> Arc<ClassMetadata> {
        Arc::clone(&self.target_entity)
    }

    fn load_collection_cache<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        key: &'a CollectionCacheKey,
    ) -> Fut<'a, Option<PhpArray>> {
        Box::pin(async move {
            let Some(cache) = self.region.get(&key.clone().into())? else {
                return Ok(None);
            };
            let CacheEntry::Collection(cache) = cache else {
                return Err(Thrown::new(
                    "TypeError",
                    "Doctrine\\ORM\\Cache\\CollectionHydrator::loadCacheEntry(): Argument #3 ($entry) must be of type Doctrine\\ORM\\Cache\\CollectionCacheEntry",
                    0,
                ));
            };
            self.hydrator
                .load_cache_entry(&self.source_entity, key, &cache, collection)
                .await
        })
    }

    fn store_collection_cache(
        &self,
        key: &CollectionCacheKey,
        elements: &PhpArray,
    ) -> Result<(), Thrown> {
        let em = self.em()?;
        let association_mapping = self
            .source_entity
            .read()
            .association_mappings
            .get(&key.association)
            .cloned()
            .ok_or_else(|| {
                Thrown::new(
                    "Error",
                    format!("Undefined array key \"{}\"", key.association),
                    0,
                )
            })?;
        let target_root = self.target_entity.read().root_entity_name.clone();
        let target_persister = em
            .get_unit_of_work()
            .get_cached_entity_persister(&target_root)?
            .ok_or_else(|| not_cached(&target_root))?;
        let target_region = target_persister.get_cache_region();
        let target_hydrator = target_persister.get_entity_hydrator();

        // Only preserve ordering if association configured it
        let elements = if association_mapping.is_indexed() {
            elements.clone()
        } else {
            // Elements may be an array or a Collection
            PhpArray::list(elements.values().cloned())
        };

        let entry = self
            .hydrator
            .build_cache_entry(&self.target_entity, key, &elements)?;

        for (index, entity_key) in &entry.identifiers {
            if target_region.contains(entity_key)? {
                continue;
            }
            let Some(Value::Object(entity)) = elements.get(index).cloned() else {
                continue;
            };
            let target_name = self.target_entity.read().name.clone();
            let class = if entity.class() == target_name {
                Arc::clone(&self.target_entity)
            } else {
                em.get_class_metadata(entity.class())?
            };
            let super::CacheKey::Entity(entity_key) = entity_key else {
                continue;
            };
            let entity_entry = target_hydrator.build_cache_entry(&class, entity_key, &entity)?;
            target_region.put(&entity_key.clone().into(), entity_entry.into(), None)?;
        }

        if self.region.put(&key.clone().into(), entry.into(), None)?
            && let Some(logger) = &self.cache_logger
        {
            logger.collection_cache_put(&self.region_name, key);
        }
        Ok(())
    }

    fn get_class_name(&self) -> &'static str {
        match self.usage {
            CacheUsage::ReadOnly => READ_ONLY_CACHED_COLLECTION_PERSISTER,
            CacheUsage::NonStrictReadWrite => NON_STRICT_READ_WRITE_CACHED_COLLECTION_PERSISTER,
            CacheUsage::ReadWrite => READ_WRITE_CACHED_COLLECTION_PERSISTER,
        }
    }
}

impl CollectionPersister for AbstractCollectionPersister {
    fn delete<'a>(&'a self, collection: &'a Arc<PersistentCollection>) -> Fut<'a, ()> {
        Box::pin(self.do_delete(collection))
    }

    fn update<'a>(&'a self, collection: &'a Arc<PersistentCollection>) -> Fut<'a, ()> {
        Box::pin(self.do_update(collection))
    }

    fn count<'a>(&'a self, collection: &'a Arc<PersistentCollection>) -> Fut<'a, i64> {
        Box::pin(self.do_count(collection))
    }

    fn slice<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        offset: i64,
        length: Option<i64>,
    ) -> Fut<'a, PhpArray> {
        self.persister.slice(collection, offset, length)
    }

    fn contains<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        element: &'a Object,
    ) -> Fut<'a, bool> {
        self.persister.contains(collection, element)
    }

    fn contains_key<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        key: &'a Key,
    ) -> Fut<'a, bool> {
        self.persister.contains_key(collection, key)
    }

    fn get<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        index: &'a Value,
    ) -> Fut<'a, Option<Object>> {
        self.persister.get(collection, index)
    }

    fn load_criteria<'a>(
        &'a self,
        collection: &'a Arc<PersistentCollection>,
        criteria: &'a Criteria,
    ) -> Fut<'a, Vec<Value>> {
        self.persister.load_criteria(collection, criteria)
    }
}
