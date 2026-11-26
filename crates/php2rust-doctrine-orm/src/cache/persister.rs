//! The second-level cache's entity persisters: the `CachedPersister` and
//! `CachedEntityPersister` interfaces, and `AbstractEntityPersister`,
//! which wraps an entity persister with a region under one of the three
//! access strategies (`ReadOnlyCachedEntityPersister`,
//! `NonStrictReadWriteCachedEntityPersister`,
//! `ReadWriteCachedEntityPersister`; [`CacheUsage`] names which, and
//! `get_class_name()` gives the upstream class).
//!
//! The collection loads go to the wrapped persister: the collection
//! persisters are not cached here (`UnitOfWork::getCollectionPersister()`
//! has no cached collection persister). `count(queuedCache)` is
//! `queued_cache_len()`. A criteria's orderings enter the query hash as
//! their `ASC`/`DESC` names.
//!
//! Upstream: doctrine/orm src/Cache/Persister/CachedPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Entity/CachedEntityPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Entity/AbstractEntityPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Entity/ReadOnlyCachedEntityPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Entity/NonStrictReadWriteCachedEntityPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/Entity/ReadWriteCachedEntityPersister.php
//! Upstream: doctrine/orm src/Cache/Persister/CompatOrderings.php

use std::fmt::Debug;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::{Criteria, SortDirection};
use php2rust_doctrine_dbal::LockMode;

use crate::entity_manager::EntityManager;
use crate::mapping::class_metadata::{
    CACHE_USAGE_NONSTRICT_READ_WRITE, CACHE_USAGE_READ_ONLY, CACHE_USAGE_READ_WRITE, FETCH_EAGER,
};
use crate::mapping::{AssociationMapping, ClassMetadata};
use crate::persistent_collection::PersistentCollection;
use crate::persisters::{
    CommitPersister, EntityPersister, EntityPersisterInterface, Params, PersisterCriteria,
};
use crate::query::ResultSetMapping;
use crate::unit_of_work::{Fut, Hints};

use super::exception::cannot_update_read_only_entity;
use super::hydrator::{EntityHydrator, em_of};
use super::logging::CacheLogger;
use super::{
    Cache, CacheEntry, EntityCacheKey, Lock, MODE_NORMAL, QueryCacheKey, Region, TimestampCacheKey,
    TimestampRegion,
};

/// `Doctrine\ORM\Cache\Persister\CachedPersister`: interface for
/// persister that support second level cache.
pub trait CachedPersister: Send + Sync {
    /// Perform whatever processing is encapsulated here after completion
    /// of the transaction.
    fn after_transaction_complete(&self) -> Result<(), Thrown>;

    /// Perform whatever processing is encapsulated here after completion
    /// of the rolled-back.
    fn after_transaction_rolled_back(&self) -> Result<(), Thrown>;

    fn get_cache_region(&self) -> Arc<dyn Region>;
}

/// `Doctrine\ORM\Cache\Persister\Entity\CachedEntityPersister`:
/// interface for second level cache entity persisters.
pub trait CachedEntityPersister: CachedPersister + EntityPersisterInterface {
    fn get_entity_hydrator(&self) -> Arc<dyn EntityHydrator>;

    fn store_entity_cache(&self, entity: &Object, key: &EntityCacheKey) -> Result<bool, Thrown>;

    /// The upstream class of this persister.
    fn get_class_name(&self) -> &'static str;
}

/// The error of an entity whose persister is not cached, where upstream
/// calls `getCacheRegion()` on it.
pub(crate) fn not_cached(entity_name: &str) -> Thrown {
    Thrown::new(
        "Error",
        format!(
            "Call to undefined method Doctrine\\ORM\\Persisters\\Entity\\BasicEntityPersister::getCacheRegion() (the persister of \"{entity_name}\" is not cached)"
        ),
        0,
    )
}

pub(crate) fn no_cache_configuration() -> Thrown {
    Thrown::new(
        "Error",
        "Call to a member function getCacheFactory() on null",
        0,
    )
}

pub(crate) fn no_cache_factory() -> Thrown {
    Thrown::new(
        "Error",
        "Call to a member function getTimestampRegion() on null",
        0,
    )
}

/// The access strategy of a cached persister.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheUsage {
    /// `ReadOnlyCachedEntityPersister` (`ClassMetadata::CACHE_USAGE_READ_ONLY`).
    ReadOnly,
    /// `NonStrictReadWriteCachedEntityPersister`.
    NonStrictReadWrite,
    /// `ReadWriteCachedEntityPersister`, over a concurrent region.
    ReadWrite,
}

impl CacheUsage {
    /// The strategy of a `ClassMetadata::CACHE_USAGE_*` value.
    pub fn from_usage(usage: i64) -> Option<Self> {
        match usage {
            CACHE_USAGE_READ_ONLY => Some(Self::ReadOnly),
            CACHE_USAGE_NONSTRICT_READ_WRITE => Some(Self::NonStrictReadWrite),
            CACHE_USAGE_READ_WRITE => Some(Self::ReadWrite),
            _ => None,
        }
    }
}

pub const READ_ONLY_CACHED_ENTITY_PERSISTER: &str =
    "Doctrine\\ORM\\Cache\\Persister\\Entity\\ReadOnlyCachedEntityPersister";
pub const NON_STRICT_READ_WRITE_CACHED_ENTITY_PERSISTER: &str =
    "Doctrine\\ORM\\Cache\\Persister\\Entity\\NonStrictReadWriteCachedEntityPersister";
pub const READ_WRITE_CACHED_ENTITY_PERSISTER: &str =
    "Doctrine\\ORM\\Cache\\Persister\\Entity\\ReadWriteCachedEntityPersister";

/// A queued `update` or `delete` of `$queuedCache`.
#[derive(Debug, Clone)]
enum Queued {
    /// The updated entity (non-strict read-write).
    Entity(Object),
    /// The deleted entity's key (non-strict read-write).
    Key(EntityCacheKey),
    /// `['lock' => $lock, 'key' => $key]` (read-write).
    Locked {
        #[allow(dead_code, reason = "the lock upstream keeps with the key")]
        lock: Lock,
        key: EntityCacheKey,
    },
}

/// `$queuedCache`: the inserts, updates and deletes of the transaction.
#[derive(Debug, Default)]
struct QueuedCache {
    insert: Option<Vec<Object>>,
    update: Option<Vec<Queued>>,
    delete: Option<Vec<Queued>>,
}

impl QueuedCache {
    fn len(&self) -> usize {
        usize::from(self.insert.is_some())
            + usize::from(self.update.is_some())
            + usize::from(self.delete.is_some())
    }
}

/// `Doctrine\ORM\Cache\Persister\Entity\AbstractEntityPersister` and its
/// three access strategies.
pub struct AbstractEntityPersister {
    usage: CacheUsage,
    persister: Arc<dyn EntityPersisterInterface>,
    region: Arc<dyn Region>,
    em: Weak<EntityManager>,
    class: Arc<ClassMetadata>,
    queued_cache: Mutex<QueuedCache>,
    timestamp_region: Arc<dyn TimestampRegion>,
    timestamp_key: TimestampCacheKey,
    hydrator: Arc<dyn EntityHydrator>,
    cache: Arc<dyn Cache>,
    cache_logger: Option<Arc<dyn CacheLogger>>,
    region_name: String,
    joined_associations: Mutex<Option<Vec<String>>>,
}

impl Debug for AbstractEntityPersister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.get_class_name())
            .field("region_name", &self.region_name)
            .finish_non_exhaustive()
    }
}

impl AbstractEntityPersister {
    /// `new ReadOnlyCachedEntityPersister(...)`,
    /// `new NonStrictReadWriteCachedEntityPersister(...)` or
    /// `new ReadWriteCachedEntityPersister(...)` (whose region is a
    /// `ConcurrentRegion`).
    pub fn new(
        usage: CacheUsage,
        persister: Arc<dyn EntityPersisterInterface>,
        region: Arc<dyn Region>,
        em: &Arc<EntityManager>,
        class: Arc<ClassMetadata>,
    ) -> Result<Self, Thrown> {
        let configuration = em.get_configuration();
        let cache_config = configuration
            .get_second_level_cache_configuration()
            .ok_or_else(no_cache_configuration)?;
        let cache_factory = cache_config
            .get_cache_factory()
            .ok_or_else(no_cache_factory)?;
        let cache = em.get_cache().ok_or_else(|| {
            Thrown::new(
                "TypeError",
                "Cannot assign null to property Doctrine\\ORM\\Cache\\Persister\\Entity\\AbstractEntityPersister::$cache of type Doctrine\\ORM\\Cache",
                0,
            )
        })?;
        let hydrator = cache_factory.build_entity_hydrator(em, &class);
        let timestamp_key = TimestampCacheKey::new(class.read().root_entity_name.clone());
        Ok(Self {
            usage,
            region_name: region.get_name(),
            persister,
            region,
            em: Arc::downgrade(em),
            class,
            queued_cache: Mutex::default(),
            timestamp_region: cache_factory.get_timestamp_region(),
            timestamp_key,
            hydrator,
            cache,
            cache_logger: cache_config.get_cache_logger(),
            joined_associations: Mutex::new(None),
        })
    }

    /// The access strategy.
    pub fn usage(&self) -> CacheUsage {
        self.usage
    }

    /// `count($this->queuedCache)`.
    pub fn queued_cache_len(&self) -> usize {
        self.queued().len()
    }

    fn queued(&self) -> MutexGuard<'_, QueuedCache> {
        self.queued_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        em_of(&self.em)
    }

    fn root_entity_name(&self) -> String {
        self.class.read().root_entity_name.clone()
    }

    fn logger(&self) -> Option<&dyn CacheLogger> {
        self.cache_logger.as_deref()
    }

    /// The metadata of an entity's class (`DefaultProxyClassNameResolver::getClass()`).
    fn class_of(&self, em: &EntityManager, entity: &Object) -> Result<Arc<ClassMetadata>, Thrown> {
        if entity.class() == self.class.read().name {
            return Ok(Arc::clone(&self.class));
        }
        em.get_class_metadata(entity.class())
    }

    fn store_joined_associations(&self, entity: &Object) -> Result<(), Thrown> {
        let joined = {
            let mut joined = self
                .joined_associations
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            joined
                .get_or_insert_with(|| {
                    self.class
                        .read()
                        .association_mappings
                        .iter()
                        .filter(|(_, assoc)| {
                            assoc.cache.is_some()
                                && assoc.is_to_one()
                                && (assoc.fetch == Some(FETCH_EAGER) || !assoc.is_owning_side())
                        })
                        .map(|(name, _)| name.to_owned())
                        .collect()
                })
                .clone()
        };
        let em = self.em()?;
        let uow = em.get_unit_of_work();
        for name in joined {
            let (target_entity, assoc_entity) = {
                let info = self.class.read();
                let Some(assoc) = info.association_mappings.get(&name) else {
                    continue;
                };
                (
                    assoc.target_entity.clone(),
                    info.get_field_value(entity, &name)?,
                )
            };
            let Value::Object(assoc_entity) = assoc_entity else {
                continue;
            };
            let assoc_id = uow.get_entity_identifier(&assoc_entity)?;
            let assoc_root = em
                .get_class_metadata(&target_entity)?
                .read()
                .root_entity_name
                .clone();
            let assoc_key = EntityCacheKey::new(assoc_root, &assoc_id);
            if let Some(assoc_persister) = uow.get_cached_entity_persister(&target_entity)? {
                assoc_persister.store_entity_cache(&assoc_entity, &assoc_key)?;
            }
        }
        Ok(())
    }

    /// `getHash()`: `sha1($query . serialize($params) . serialize($orderBy)
    /// . $limit . $offset . $filters->getHash())`.
    fn get_hash(
        &self,
        query: &str,
        criteria: PersisterCriteria<'_>,
        order_by: Option<&PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<String, Thrown> {
        let (params, _) = match criteria {
            PersisterCriteria::Criteria(criteria) => {
                self.persister.expand_criteria_parameters(criteria)?
            }
            PersisterCriteria::Array(criteria) => self.persister.expand_parameters(criteria)?,
        };
        let em = self.em()?;
        let mut data = query.as_bytes().to_vec();
        data.extend_from_slice(
            php_value::serialize::serialize(&Value::Array(PhpArray::list(params))).as_bytes(),
        );
        data.extend_from_slice(
            php_value::serialize::serialize(
                &order_by.map_or(Value::Null, |o| Value::Array(o.clone())),
            )
            .as_bytes(),
        );
        data.extend_from_slice(limit.map(|l| l.to_string()).unwrap_or_default().as_bytes());
        data.extend_from_slice(offset.map(|o| o.to_string()).unwrap_or_default().as_bytes());
        data.extend_from_slice(em.get_filters().get_hash().as_bytes());
        Ok(sha1_hex(&data))
    }

    fn query_key(&self, hash: String) -> QueryCacheKey {
        QueryCacheKey::with(hash, 0, MODE_NORMAL, Some(self.timestamp_key.clone()))
    }

    fn entity_key(&self, entity: &Object) -> Result<EntityCacheKey, Thrown> {
        let em = self.em()?;
        let identifier = em.get_unit_of_work().get_entity_identifier(entity)?;
        Ok(EntityCacheKey::new(self.root_entity_name(), &identifier))
    }

    fn update_cache(&self, entity: &Object, is_changed: bool) -> Result<bool, Thrown> {
        let em = self.em()?;
        let class = em.get_class_metadata(entity.class())?;
        let root = class.read().root_entity_name.clone();
        let identifier = em.get_unit_of_work().get_entity_identifier(entity)?;
        let key = EntityCacheKey::new(root, &identifier);
        let entry = self.hydrator.build_cache_entry(&class, &key, entity)?;
        let cached = self.region.put(&key.clone().into(), entry.into(), None)?;
        if cached && let Some(logger) = self.logger() {
            logger.entity_cache_put(&self.region_name, &key);
        }
        Ok(is_changed || cached)
    }

    fn concurrent_lock(&self, key: &EntityCacheKey) -> Result<Option<Lock>, Thrown> {
        let Some(region) = self.region.as_concurrent() else {
            return Err(Thrown::new(
                "TypeError",
                "ReadWriteCachedEntityPersister::__construct(): Argument #2 ($region) must be of type Doctrine\\ORM\\Cache\\ConcurrentRegion",
                0,
            ));
        };
        region.lock(&key.clone().into())
    }

    /// `orderings()` of a criteria, as a PHP array of `ASC`/`DESC`.
    fn get_orderings(criteria: &Criteria) -> PhpArray {
        criteria
            .get_orderings()
            .into_iter()
            .map(|(field, direction)| {
                let name = match direction {
                    SortDirection::Ascending => "ASC",
                    SortDirection::Descending => "DESC",
                };
                (field, Value::string(name))
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments, reason = "load(), as PHP declares it")]
    async fn do_load(
        &self,
        criteria: &PhpArray,
        entity: Option<&Object>,
        assoc: Option<&AssociationMapping>,
        hints: Hints,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<Option<Object>, Thrown> {
        let hints_given = hints.refresh
            || hints.refresh_entity.is_some()
            || hints.read_only
            || hints.defer_eager_load
            || hints.fetch_alias.is_some()
            || !hints.fetched.is_empty()
            || !hints.fetch_mode.is_empty()
            || hints.cache_enabled;
        if entity.is_some() || assoc.is_some() || hints_given || lock_mode.is_some() {
            return self
                .persister
                .load(criteria, entity, assoc, hints, lock_mode, limit, order_by)
                .await;
        }

        //handle only EntityRepository#findOneBy
        let query =
            self.persister
                .get_select_sql(criteria.into(), None, None, limit, None, order_by)?;
        let hash = self.get_hash(&query, criteria.into(), None, None, None)?;
        let rsm = self.get_result_set_mapping();
        let query_key = self.query_key(hash);
        let query_cache = self.cache.get_query_cache(Some(&self.region_name))?;
        let no_hints = PhpArray::new();
        if let Some(result) = query_cache.get(&query_key, &rsm, &no_hints).await? {
            if let Some(logger) = self.logger() {
                logger.query_cache_hit(&self.region_name, &query_key);
            }
            return Ok(result.values().next().and_then(|v| match v {
                Value::Object(o) => Some(o.clone()),
                _ => None,
            }));
        }

        let Some(result) = self
            .persister
            .load(criteria, entity, assoc, hints, lock_mode, limit, order_by)
            .await?
        else {
            return Ok(None);
        };

        let list = PhpArray::list([Value::Object(result.clone())]);
        let cached = query_cache.put(&query_key, &rsm, &list, &no_hints).await?;
        if let Some(logger) = self.logger() {
            logger.query_cache_miss(&self.region_name, &query_key);
            if cached {
                logger.query_cache_put(&self.region_name, &query_key);
            }
        }
        Ok(Some(result))
    }

    async fn cached_list(
        &self,
        query_key: &QueryCacheKey,
        rsm: &ResultSetMapping,
        load: Fut<'_, Vec<Object>>,
    ) -> Result<Vec<Object>, Thrown> {
        let query_cache = self.cache.get_query_cache(Some(&self.region_name))?;
        let no_hints = PhpArray::new();
        if let Some(result) = query_cache.get(query_key, rsm, &no_hints).await? {
            if let Some(logger) = self.logger() {
                logger.query_cache_hit(&self.region_name, query_key);
            }
            return Ok(result
                .values()
                .filter_map(|v| match v {
                    Value::Object(o) => Some(o.clone()),
                    _ => None,
                })
                .collect());
        }

        let result = load.await?;
        let list = PhpArray::list(result.iter().cloned().map(Value::Object));
        let cached = query_cache.put(query_key, rsm, &list, &no_hints).await?;
        if let Some(logger) = self.logger() {
            if !result.is_empty() {
                logger.query_cache_miss(&self.region_name, query_key);
            }
            if cached {
                logger.query_cache_put(&self.region_name, query_key);
            }
        }
        Ok(result)
    }

    async fn do_load_by_id(
        &self,
        identifier: &PhpArray,
        entity: Option<&Object>,
    ) -> Result<Option<Object>, Thrown> {
        let em = self.em()?;
        let cache_key = EntityCacheKey::new(self.root_entity_name(), identifier);
        let cache_entry = self.region.get(&cache_key.clone().into())?;

        if let Some(CacheEntry::Entity(cache_entry)) = cache_entry {
            let class = if cache_entry.class == self.class.read().name {
                Arc::clone(&self.class)
            } else {
                em.get_class_metadata(&cache_entry.class)?
            };
            let cached_entity = self
                .hydrator
                .load_cache_entry(&class, &cache_key, &cache_entry, entity)
                .await?;
            if let Some(cached_entity) = cached_entity {
                if let Some(logger) = self.logger() {
                    logger.entity_cache_hit(&self.region_name, &cache_key);
                }
                return Ok(Some(cached_entity));
            }
        }

        let Some(entity) = self.persister.load_by_id(identifier, entity).await? else {
            return Ok(None);
        };

        let class = self.class_of(&em, &entity)?;
        let cache_entry = self
            .hydrator
            .build_cache_entry(&class, &cache_key, &entity)?;
        let cached = self
            .region
            .put(&cache_key.clone().into(), cache_entry.into(), None)?;

        let joined_pending = self
            .joined_associations
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_none_or(|joined| !joined.is_empty());
        if cached && joined_pending {
            self.store_joined_associations(&entity)?;
        }

        if let Some(logger) = self.logger() {
            if cached {
                logger.entity_cache_put(&self.region_name, &cache_key);
            }
            logger.entity_cache_miss(&self.region_name, &cache_key);
        }
        Ok(Some(entity))
    }

    async fn do_delete(&self, entity: &Object) -> Result<bool, Thrown> {
        let key = self.entity_key(entity)?;
        if self.usage == CacheUsage::ReadWrite {
            let lock = self.concurrent_lock(&key)?;
            let deleted = self.persister.delete(entity).await?;
            if deleted {
                self.region.evict(&key.clone().into())?;
            }
            let Some(lock) = lock else {
                return Ok(deleted);
            };
            self.queued()
                .delete
                .get_or_insert_with(Vec::new)
                .push(Queued::Locked { lock, key });
            return Ok(deleted);
        }
        let deleted = self.persister.delete(entity).await?;
        if deleted {
            self.region.evict(&key.clone().into())?;
        }
        self.queued()
            .delete
            .get_or_insert_with(Vec::new)
            .push(Queued::Key(key));
        Ok(deleted)
    }

    async fn do_update(&self, entity: &Object) -> Result<(), Thrown> {
        match self.usage {
            CacheUsage::ReadOnly => Err(cannot_update_read_only_entity(entity.class())),
            CacheUsage::NonStrictReadWrite => {
                self.persister.update(entity).await?;
                self.queued()
                    .update
                    .get_or_insert_with(Vec::new)
                    .push(Queued::Entity(entity.clone()));
                Ok(())
            }
            CacheUsage::ReadWrite => {
                let key = self.entity_key(entity)?;
                let lock = self.concurrent_lock(&key)?;
                self.persister.update(entity).await?;
                let Some(lock) = lock else {
                    return Ok(());
                };
                self.queued()
                    .update
                    .get_or_insert_with(Vec::new)
                    .push(Queued::Locked { lock, key });
                Ok(())
            }
        }
    }

    fn evict_queued(&self, items: &[Queued]) -> Result<(), Thrown> {
        for item in items {
            if let Queued::Locked { key, .. } | Queued::Key(key) = item {
                self.region.evict(&key.clone().into())?;
            }
        }
        Ok(())
    }
}

impl CachedPersister for AbstractEntityPersister {
    fn after_transaction_complete(&self) -> Result<(), Thrown> {
        let queued = std::mem::take(&mut *self.queued());
        if self.usage == CacheUsage::ReadWrite {
            let mut is_changed = true;
            if let Some(update) = &queued.update {
                self.evict_queued(update)?;
                is_changed = true;
            }
            if let Some(delete) = &queued.delete {
                self.evict_queued(delete)?;
                is_changed = true;
            }
            if is_changed {
                self.timestamp_region
                    .update(&self.timestamp_key.clone().into())?;
            }
            return Ok(());
        }

        let mut is_changed = false;
        for entity in queued.insert.iter().flatten() {
            is_changed = self.update_cache(entity, is_changed)?;
        }
        for item in queued.update.iter().flatten() {
            if let Queued::Entity(entity) = item {
                is_changed = self.update_cache(entity, is_changed)?;
            }
        }
        if let Some(delete) = &queued.delete {
            self.evict_queued(delete)?;
            is_changed = true;
        }
        if is_changed {
            self.timestamp_region
                .update(&self.timestamp_key.clone().into())?;
        }
        Ok(())
    }

    fn after_transaction_rolled_back(&self) -> Result<(), Thrown> {
        let queued = std::mem::take(&mut *self.queued());
        if self.usage == CacheUsage::ReadWrite {
            if let Some(update) = &queued.update {
                self.evict_queued(update)?;
            }
            if let Some(delete) = &queued.delete {
                self.evict_queued(delete)?;
            }
        }
        Ok(())
    }

    fn get_cache_region(&self) -> Arc<dyn Region> {
        Arc::clone(&self.region)
    }
}

impl CachedEntityPersister for AbstractEntityPersister {
    fn get_entity_hydrator(&self) -> Arc<dyn EntityHydrator> {
        Arc::clone(&self.hydrator)
    }

    fn store_entity_cache(&self, entity: &Object, key: &EntityCacheKey) -> Result<bool, Thrown> {
        let em = self.em()?;
        let class = self.class_of(&em, entity)?;
        let entry = self.hydrator.build_cache_entry(&class, key, entity)?;
        let cached = self.region.put(&key.clone().into(), entry.into(), None)?;
        if cached && let Some(logger) = self.logger() {
            logger.entity_cache_put(&self.region_name, key);
        }
        Ok(cached)
    }

    fn get_class_name(&self) -> &'static str {
        match self.usage {
            CacheUsage::ReadOnly => READ_ONLY_CACHED_ENTITY_PERSISTER,
            CacheUsage::NonStrictReadWrite => NON_STRICT_READ_WRITE_CACHED_ENTITY_PERSISTER,
            CacheUsage::ReadWrite => READ_WRITE_CACHED_ENTITY_PERSISTER,
        }
    }
}

impl EntityPersister for AbstractEntityPersister {
    fn count_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, i64> {
        self.persister.count_criteria(criteria)
    }

    /// `loadCriteria()`: through the query cache.
    fn load_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, Vec<Value>> {
        Box::pin(async move {
            let order_by = Self::get_orderings(criteria);
            let limit = criteria.get_max_results();
            let offset = criteria.get_first_result();
            let query =
                self.persister
                    .get_select_sql(criteria.into(), None, None, None, None, None)?;
            let hash = self.get_hash(&query, criteria.into(), Some(&order_by), limit, offset)?;
            let rsm = self.get_result_set_mapping();
            let query_key = self.query_key(hash);
            let load: Fut<'_, Vec<Object>> = Box::pin(async move {
                Ok(
                    EntityPersister::load_criteria(self.persister.as_ref(), criteria)
                        .await?
                        .into_iter()
                        .filter_map(|v| match v {
                            Value::Object(o) => Some(o),
                            _ => None,
                        })
                        .collect(),
                )
            });
            Ok(self
                .cached_list(&query_key, &rsm, load)
                .await?
                .into_iter()
                .map(Value::Object)
                .collect())
        })
    }

    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool> {
        CommitPersister::exists(self, entity, extra_conditions)
    }

    fn get_select_condition_statement_sql(
        &self,
        field: &str,
        value: &Value,
        assoc: Option<&AssociationMapping>,
        comparison: Option<&str>,
    ) -> Result<String, Thrown> {
        self.persister
            .get_select_condition_statement_sql(field, value, assoc, comparison)
    }
}

impl CommitPersister for AbstractEntityPersister {
    fn add_insert(&self, entity: &Object) {
        self.persister.add_insert(entity);
    }

    fn execute_inserts(&self) -> Fut<'_, ()> {
        Box::pin(async move {
            // The commit order/foreign key relationships may make it
            // necessary that multiple calls to executeInsert() are
            // performed, so collect all the new entities.
            let new_inserts = self.persister.get_inserts();
            if !new_inserts.is_empty() {
                self.queued()
                    .insert
                    .get_or_insert_with(Vec::new)
                    .extend(new_inserts);
            }
            self.persister.execute_inserts().await
        })
    }

    fn update<'a>(&'a self, entity: &'a Object) -> Fut<'a, ()> {
        Box::pin(self.do_update(entity))
    }

    fn delete<'a>(&'a self, entity: &'a Object) -> Fut<'a, bool> {
        Box::pin(self.do_delete(entity))
    }

    fn exists<'a>(
        &'a self,
        entity: &'a Object,
        extra_conditions: Option<&'a Criteria>,
    ) -> Fut<'a, bool> {
        Box::pin(async move {
            if extra_conditions.is_none() {
                let identifier = self.class.read().get_identifier_values(entity)?;
                let key = EntityCacheKey::new(self.root_entity_name(), &identifier);
                if self.region.contains(&key.into())? {
                    return Ok(true);
                }
            }
            CommitPersister::exists(self.persister.as_ref(), entity, extra_conditions).await
        })
    }
}

impl EntityPersisterInterface for AbstractEntityPersister {
    fn get_class_metadata(&self) -> Arc<ClassMetadata> {
        self.persister.get_class_metadata()
    }

    fn get_result_set_mapping(&self) -> ResultSetMapping {
        self.persister.get_result_set_mapping()
    }

    fn get_inserts(&self) -> Vec<Object> {
        self.persister.get_inserts()
    }

    fn get_select_sql(
        &self,
        criteria: PersisterCriteria<'_>,
        assoc: Option<&AssociationMapping>,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        offset: Option<i64>,
        order_by: Option<&PhpArray>,
    ) -> Result<String, Thrown> {
        self.persister
            .get_select_sql(criteria, assoc, lock_mode, limit, offset, order_by)
    }

    fn get_count_sql(&self, criteria: PersisterCriteria<'_>) -> Result<String, Thrown> {
        self.persister.get_count_sql(criteria)
    }

    fn get_insert_sql(&self) -> Result<String, Thrown> {
        self.persister.get_insert_sql()
    }

    fn expand_parameters(&self, criteria: &PhpArray) -> Result<Params, Thrown> {
        self.persister.expand_parameters(criteria)
    }

    fn expand_criteria_parameters(&self, criteria: &Criteria) -> Result<Params, Thrown> {
        self.persister.expand_criteria_parameters(criteria)
    }

    fn get_owning_table(&self, field_name: &str) -> Result<String, Thrown> {
        self.persister.get_owning_table(field_name)
    }

    fn load<'a>(
        &'a self,
        criteria: &'a PhpArray,
        entity: Option<&'a Object>,
        assoc: Option<&'a AssociationMapping>,
        hints: Hints,
        lock_mode: Option<LockMode>,
        limit: Option<i64>,
        order_by: Option<&'a PhpArray>,
    ) -> Fut<'a, Option<Object>> {
        Box::pin(self.do_load(criteria, entity, assoc, hints, lock_mode, limit, order_by))
    }

    fn load_by_id<'a>(
        &'a self,
        identifier: &'a PhpArray,
        entity: Option<&'a Object>,
    ) -> Fut<'a, Option<Object>> {
        Box::pin(self.do_load_by_id(identifier, entity))
    }

    fn load_all<'a>(
        &'a self,
        criteria: &'a PhpArray,
        order_by: Option<&'a PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Fut<'a, Vec<Object>> {
        Box::pin(async move {
            let query = self.persister.get_select_sql(
                criteria.into(),
                None,
                None,
                limit,
                offset,
                order_by,
            )?;
            let hash = self.get_hash(&query, criteria.into(), None, None, None)?;
            let rsm = self.get_result_set_mapping();
            let query_key = self.query_key(hash);
            let load = self.persister.load_all(criteria, order_by, limit, offset);
            self.cached_list(&query_key, &rsm, load).await
        })
    }

    fn load_one_to_one_entity<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        identifier: Option<&'a PhpArray>,
    ) -> Fut<'a, Option<Object>> {
        self.persister
            .load_one_to_one_entity(assoc, source_entity, identifier)
    }

    fn refresh<'a>(
        &'a self,
        id: &'a PhpArray,
        entity: &'a Object,
        lock_mode: Option<LockMode>,
    ) -> Fut<'a, ()> {
        self.persister.refresh(id, entity, lock_mode)
    }

    fn count<'a>(&'a self, criteria: &'a PhpArray) -> Fut<'a, i64> {
        self.persister.count(criteria)
    }

    fn lock<'a>(&'a self, criteria: &'a PhpArray, lock_mode: LockMode) -> Fut<'a, ()> {
        self.persister.lock(criteria, lock_mode)
    }

    fn get_many_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Fut<'a, PhpArray> {
        self.persister
            .get_many_to_many_collection(assoc, source_entity, offset, limit)
    }

    fn load_many_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Vec<Object>> {
        self.persister
            .load_many_to_many_collection(assoc, source_entity, collection)
    }

    fn get_one_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        offset: Option<i64>,
        limit: Option<i64>,
    ) -> Fut<'a, PhpArray> {
        self.persister
            .get_one_to_many_collection(assoc, source_entity, offset, limit)
    }

    fn load_one_to_many_collection<'a>(
        &'a self,
        assoc: &'a AssociationMapping,
        source_entity: &'a Object,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Vec<Object>> {
        self.persister
            .load_one_to_many_collection(assoc, source_entity, collection)
    }
}

/// `sha1($data)`: the SHA-1 digest of a byte string, in lowercase hex.
fn sha1_hex(data: &[u8]) -> String {
    use sha1::Digest as _;
    use std::fmt::Write as _;
    sha1::Sha1::digest(data)
        .iter()
        .fold(String::with_capacity(40), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        })
}
