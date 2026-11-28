//! The query result cache: the `QueryCache` interface and
//! `DefaultQueryCache`, which keeps a query's entity identifiers (and
//! its fetched associations') in a region and the entities in their
//! entity regions.
//!
//! The results are PHP arrays of entities; the hints are a PHP array of
//! query hints.
//!
//! Upstream: doctrine/orm src/Cache/QueryCache.php src/Cache/DefaultQueryCache.php

use std::fmt::Debug;
use std::sync::{Arc, Weak};

use php_value::{Key, Object, PhpArray, Thrown, Value};

use php2rust_doctrine_collections::ArrayCollection;

use crate::collection_object::ArrayCollectionObject;
use crate::entity_manager::EntityManager;
use crate::mapping::class_metadata::TO_ONE;
use crate::persistent_collection::PersistentCollection;
use crate::query::ResultSetMapping;
use crate::query::context::{HINT_FORCE_PARTIAL_LOAD, HINT_PARTIAL};
use crate::unit_of_work::Fut;

use super::exception::{
    multiple_root_entities, non_cacheable_entity, non_select_statements, partial_entities,
    scalar_results,
};
use super::hydrator::{cache_hints, em_of, resolve_association_entries};
use super::logging::CacheLogger;
use super::persister::{CachedEntityPersister, not_cached};
use super::{
    AssociationCacheEntry, CacheEntry, CollectionCacheEntry, EntityCacheKey, MODE_GET, MODE_PUT,
    MODE_REFRESH, QueryCacheEntry, QueryCacheKey, QueryCacheValidator, Region,
};

/// `Doctrine\ORM\Cache\QueryCache`: defines the contract for caches
/// capable of storing query results.
pub trait QueryCache: Debug + Send + Sync {
    fn clear(&self) -> Result<bool, Thrown>;

    /// Stores a query result (a PHP array of entities).
    fn put<'a>(
        &'a self,
        key: &'a QueryCacheKey,
        rsm: &'a ResultSetMapping,
        result: &'a PhpArray,
        hints: &'a PhpArray,
    ) -> Fut<'a, bool>;

    /// The cached result, or `None`.
    fn get<'a>(
        &'a self,
        key: &'a QueryCacheKey,
        rsm: &'a ResultSetMapping,
        hints: &'a PhpArray,
    ) -> Fut<'a, Option<PhpArray>>;

    fn get_region(&self) -> Arc<dyn Region>;
}

/// `$array[$name]`.
fn field(array: &PhpArray, name: &str) -> Value {
    array.get(&Key::from(name)).cloned().unwrap_or(Value::Null)
}

fn as_array(value: &Value) -> PhpArray {
    match value {
        Value::Array(array) => array.clone(),
        _ => PhpArray::new(),
    }
}

/// The elements of a collection value (a `PersistentCollection` or an
/// array), as `foreach` iterates them.
fn elements(value: &Value) -> PhpArray {
    if let Some(collection) = PersistentCollection::from_value(value) {
        return collection.unwrap();
    }
    if let Some(collection) = ArrayCollectionObject::from_value(value) {
        return collection.lock().to_array();
    }
    as_array(value)
}

/// `Doctrine\ORM\Cache\DefaultQueryCache`: default query cache
/// implementation.
#[derive(Debug)]
pub struct DefaultQueryCache {
    em: Weak<EntityManager>,
    region: Arc<dyn Region>,
    validator: Arc<dyn QueryCacheValidator>,
    cache_logger: Option<Arc<dyn CacheLogger>>,
}

impl DefaultQueryCache {
    /// `new DefaultQueryCache($em, $region)`.
    pub fn new(em: &Arc<EntityManager>, region: Arc<dyn Region>) -> Result<Self, Thrown> {
        let cache_config = em
            .get_configuration()
            .get_second_level_cache_configuration()
            .ok_or_else(super::persister::no_cache_configuration)?;
        let validator = cache_config
            .get_query_validator()
            .ok_or_else(super::persister::no_cache_factory)?;
        Ok(Self {
            em: Arc::downgrade(em),
            region,
            validator,
            cache_logger: cache_config.get_cache_logger(),
        })
    }

    fn persister(
        em: &EntityManager,
        entity_name: &str,
    ) -> Result<Arc<dyn CachedEntityPersister>, Thrown> {
        em.get_unit_of_work()
            .get_cached_entity_persister(entity_name)?
            .ok_or_else(|| not_cached(entity_name))
    }

    async fn create(
        em: &EntityManager,
        entry: &super::EntityCacheEntry,
        data: Option<PhpArray>,
    ) -> Result<Object, Thrown> {
        let data = match data {
            Some(data) => data,
            None => resolve_association_entries(entry, em).await?,
        };
        let mut hints = cache_hints();
        em.get_unit_of_work()
            .create_entity(&entry.class, data, &mut hints)
            .await
    }

    #[allow(clippy::too_many_lines, reason = "get(), in its order")]
    async fn do_get(
        &self,
        key: &QueryCacheKey,
        rsm: &ResultSetMapping,
    ) -> Result<Option<PhpArray>, Thrown> {
        if key.cache_mode & MODE_GET == 0 {
            return Ok(None);
        }
        let Some(CacheEntry::Query(cache_entry)) = self.region.get(&key.clone().into())? else {
            return Ok(None);
        };
        if !self.validator.is_valid(key, &cache_entry)? {
            self.region.evict(&key.clone().into())?;
            return Ok(None);
        }

        let em = em_of(&self.em)?;
        let uow = em.get_unit_of_work();
        let logger = self.cache_logger.as_deref();
        let mut result = PhpArray::new();
        let entity_name = rsm.alias_map.values().next().cloned().unwrap_or_default();
        let has_relation = !rsm.relation_map.is_empty();
        let persister = Self::persister(&em, &entity_name)?;
        let region = persister.get_cache_region();
        let region_name = region.get_name();

        let root = em
            .get_class_metadata(&entity_name)?
            .read()
            .root_entity_name
            .clone();
        let cache_keys = CollectionCacheEntry::new(
            cache_entry
                .result
                .iter()
                .map(|(index, entry)| {
                    let identifier = as_array(&field(&as_array(entry), "identifier"));
                    (
                        index.clone(),
                        EntityCacheKey::new(root.as_str(), &identifier).into(),
                    )
                })
                .collect(),
        );
        let entries = region.get_multiple(&cache_keys)?.unwrap_or_default();

        // @TODO - move to cache hydration component
        for ((index, entry), (_, entity_key)) in
            cache_entry.result.iter().zip(&cache_keys.identifiers)
        {
            let super::CacheKey::Entity(entity_key) = entity_key else {
                continue;
            };
            let entity_entry = entries.iter().find(|(k, _)| k == index).map(|(_, e)| e);
            let Some(CacheEntry::Entity(entity_entry)) = entity_entry else {
                if let Some(logger) = logger {
                    logger.entity_cache_miss(&region_name, entity_key);
                }
                return Ok(None);
            };

            if let Some(logger) = logger {
                logger.entity_cache_hit(&region_name, entity_key);
            }

            if !has_relation {
                let entity = Self::create(&em, entity_entry, None).await?;
                result.insert(index.clone(), Value::Object(entity));
                continue;
            }

            let mut data = entity_entry.data.clone();
            let entry = as_array(entry);

            for (name, assoc) in &as_array(&field(&entry, "associations")) {
                let assoc = as_array(assoc);
                let target_entity = crate::utility::to_text(&field(&assoc, "targetEntity"));
                let assoc_persister = Self::persister(&em, &target_entity)?;
                let assoc_region = assoc_persister.get_cache_region();
                let assoc_root = em
                    .get_class_metadata(&target_entity)?
                    .read()
                    .root_entity_name
                    .clone();
                let assoc_type = match field(&assoc, "type") {
                    Value::Int(t) => t,
                    _ => 0,
                };

                if assoc_type & TO_ONE != 0 {
                    let assoc_key = EntityCacheKey::new(
                        assoc_root.as_str(),
                        &as_array(&field(&assoc, "identifier")),
                    );
                    let Some(CacheEntry::Entity(assoc_entry)) =
                        assoc_region.get(&assoc_key.clone().into())?
                    else {
                        if let Some(logger) = logger {
                            logger.entity_cache_miss(&assoc_region.get_name(), &assoc_key);
                        }
                        uow.hydration_complete()?;
                        return Ok(None);
                    };
                    let element = Self::create(&em, &assoc_entry, None).await?;
                    data.insert(name.clone(), Value::Object(element));
                    if let Some(logger) = logger {
                        logger.entity_cache_hit(&assoc_region.get_name(), &assoc_key);
                    }
                    continue;
                }

                let list = as_array(&field(&assoc, "list"));
                if list.is_empty() {
                    continue;
                }

                let assoc_metadata = em.get_class_metadata(&target_entity)?;
                let collection = PersistentCollection::new(
                    Some(&em),
                    Some(assoc_metadata),
                    ArrayCollection::new(PhpArray::new()),
                );
                let assoc_keys = CollectionCacheEntry::new(
                    list.iter()
                        .map(|(i, id)| {
                            (
                                i.clone(),
                                EntityCacheKey::new(assoc_root.as_str(), &as_array(id)).into(),
                            )
                        })
                        .collect(),
                );
                let assoc_entries = assoc_region.get_multiple(&assoc_keys)?;

                for ((assoc_index, _), (_, assoc_key)) in list.iter().zip(&assoc_keys.identifiers) {
                    let super::CacheKey::Entity(assoc_key) = assoc_key else {
                        continue;
                    };
                    let assoc_entry = assoc_entries
                        .as_ref()
                        .and_then(|entries| entries.iter().find(|(k, _)| k == assoc_index))
                        .map(|(_, e)| e);
                    let Some(CacheEntry::Entity(assoc_entry)) = assoc_entry else {
                        if let Some(logger) = logger {
                            logger.entity_cache_miss(&assoc_region.get_name(), assoc_key);
                        }
                        uow.hydration_complete()?;
                        return Ok(None);
                    };
                    let element = Self::create(&em, assoc_entry, None).await?;
                    collection.hydrate_set(assoc_index.clone(), Value::Object(element))?;
                    if let Some(logger) = logger {
                        logger.entity_cache_hit(&assoc_region.get_name(), assoc_key);
                    }
                }

                data.insert(name.clone(), collection.value());
                collection.set_initialized(true);
            }

            // Unwrap the associations that are not cached (they hold
            // AssociationCacheEntry objects) into proxy references.
            let fields: Vec<(Key, Value)> =
                data.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            for (field_name, value) in fields {
                if let Some(association) = AssociationCacheEntry::from_value(&value) {
                    let reference = em
                        .get_reference(
                            &association.class,
                            Value::Array(association.identifier.clone()),
                        )
                        .await?;
                    data.insert(field_name, reference.map_or(Value::Null, Value::Object));
                }
            }

            let entity = Self::create(&em, entity_entry, Some(data)).await?;
            result.insert(index.clone(), Value::Object(entity));
        }

        uow.hydration_complete()?;
        Ok(Some(result))
    }

    #[allow(clippy::too_many_lines, reason = "put(), in its order")]
    fn do_put(
        &self,
        key: &QueryCacheKey,
        rsm: &ResultSetMapping,
        result: &PhpArray,
        hints: &PhpArray,
    ) -> Result<bool, Thrown> {
        if !rsm.scalar_mappings.is_empty() {
            return Err(scalar_results());
        }
        if rsm.entity_mappings.len() > 1 {
            return Err(multiple_root_entities());
        }
        if !rsm.is_select {
            return Err(non_select_statements());
        }
        if field(hints, HINT_PARTIAL) == Value::Bool(true)
            || field(hints, HINT_FORCE_PARTIAL_LOAD) == Value::Bool(true)
        {
            return Err(partial_entities());
        }
        if key.cache_mode & MODE_PUT == 0 && key.cache_mode & MODE_REFRESH == 0 {
            return Ok(false);
        }

        let em = em_of(&self.em)?;
        let uow = em.get_unit_of_work();
        let mut data = PhpArray::new();
        let (root_alias, entity_name) = rsm
            .alias_map
            .iter()
            .next()
            .map_or_default(|(alias, name)| (alias.to_owned(), name.clone()));
        let persister = uow
            .get_cached_entity_persister(&entity_name)?
            .ok_or_else(|| non_cacheable_entity(&entity_name))?;
        let region = persister.get_cache_region();
        let root = em
            .get_class_metadata(&entity_name)?
            .read()
            .root_entity_name
            .clone();

        for (index, entity) in result {
            let Value::Object(entity) = entity else {
                continue;
            };
            let identifier = uow.get_entity_identifier(entity)?;
            let entity_key = EntityCacheKey::new(root.as_str(), &identifier);

            if (key.cache_mode & MODE_REFRESH != 0
                || !region.contains(&entity_key.clone().into())?)
                && !persister.store_entity_cache(entity, &entity_key)?
            {
                // Cancel put result if entity put fail
                return Ok(false);
            }

            let mut item = PhpArray::new();
            item.insert("identifier", Value::Array(identifier));
            let mut associations = PhpArray::new();

            // @TODO - move to cache hydration components
            for (alias, name) in rsm.relation_map.iter() {
                let parent_alias = rsm.parent_alias_map.get(alias).cloned().unwrap_or_default();
                let parent_class = rsm
                    .alias_map
                    .get(&parent_alias)
                    .cloned()
                    .unwrap_or_default();
                let metadata = em.get_class_metadata(&parent_class)?;
                let Some(assoc) = metadata.read().association_mappings.get(name).cloned() else {
                    continue;
                };
                let Some(assoc_value) = self.get_association_value(&em, rsm, alias, entity)? else {
                    continue;
                };

                // root entity association
                if root_alias == parent_alias {
                    // Cancel put result if association put fail
                    let Some(assoc_info) =
                        self.store_association_cache(&em, key, &assoc, &assoc_value)?
                    else {
                        return Ok(false);
                    };
                    associations.insert(name.as_str(), Value::Array(assoc_info));
                    continue;
                }

                // store single nested association
                if !matches!(assoc_value, Value::Array(_)) {
                    // Cancel put result if association put fail
                    if self
                        .store_association_cache(&em, key, &assoc, &assoc_value)?
                        .is_none()
                    {
                        return Ok(false);
                    }
                    continue;
                }

                // store array of nested association
                for value in elements(&assoc_value).values() {
                    // Cancel put result if association put fail
                    if self
                        .store_association_cache(&em, key, &assoc, value)?
                        .is_none()
                    {
                        return Ok(false);
                    }
                }
            }

            item.insert("associations", Value::Array(associations));
            data.insert(index.clone(), Value::Array(item));
        }

        self.region.put(
            &key.clone().into(),
            QueryCacheEntry::new(data, None).into(),
            None,
        )
    }

    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    fn store_association_cache(
        &self,
        em: &EntityManager,
        key: &QueryCacheKey,
        assoc: &crate::mapping::AssociationMapping,
        assoc_value: &Value,
    ) -> Result<Option<PhpArray>, Thrown> {
        let uow = em.get_unit_of_work();
        let assoc_persister = Self::persister(em, &assoc.target_entity)?;
        let assoc_root = assoc_persister
            .get_class_metadata()
            .read()
            .root_entity_name
            .clone();
        let assoc_region = assoc_persister.get_cache_region();

        // Handle *-to-one associations
        if assoc.is_to_one() {
            let Value::Object(object) = assoc_value else {
                return Ok(None);
            };
            let assoc_identifier = uow.get_entity_identifier(object)?;
            let entity_key = EntityCacheKey::new(assoc_root.as_str(), &assoc_identifier);
            if ((!uow.is_uninitialized_object(assoc_value) && key.cache_mode & MODE_REFRESH != 0)
                || !assoc_region.contains(&entity_key.clone().into())?)
                && !assoc_persister.store_entity_cache(object, &entity_key)?
            {
                // Entity put fail
                return Ok(None);
            }
            let mut info = PhpArray::new();
            info.insert("targetEntity", Value::string(assoc_root.as_str()));
            info.insert("identifier", Value::Array(assoc_identifier));
            info.insert("type", Value::Int(assoc.r#type()?));
            return Ok(Some(info));
        }

        // Handle *-to-many associations
        let mut list = PhpArray::new();
        for (assoc_item_index, assoc_item) in &elements(assoc_value) {
            let Value::Object(item) = assoc_item else {
                continue;
            };
            let assoc_identifier = uow.get_entity_identifier(item)?;
            let entity_key = EntityCacheKey::new(assoc_root.as_str(), &assoc_identifier);
            if (key.cache_mode & MODE_REFRESH != 0
                || !assoc_region.contains(&entity_key.clone().into())?)
                && !assoc_persister.store_entity_cache(item, &entity_key)?
            {
                // Entity put fail
                return Ok(None);
            }
            list.insert(assoc_item_index.clone(), Value::Array(assoc_identifier));
        }
        let mut info = PhpArray::new();
        info.insert("targetEntity", Value::string(assoc_root.as_str()));
        info.insert("type", Value::Int(assoc.r#type()?));
        info.insert("list", Value::Array(list));
        Ok(Some(info))
    }

    /// `getAssociationValue()` (private upstream; its test calls it by
    /// reflection).
    #[allow(clippy::unused_self, reason = "an instance method upstream")]
    pub fn get_association_value(
        &self,
        em: &EntityManager,
        rsm: &ResultSetMapping,
        assoc_alias: &str,
        entity: &Object,
    ) -> Result<Option<Value>, Thrown> {
        let mut path = Vec::new();
        let mut alias = assoc_alias.to_owned();
        while let Some(parent) = rsm.parent_alias_map.get(&alias) {
            let field = rsm.relation_map.get(&alias).cloned().unwrap_or_default();
            let class = rsm.alias_map.get(parent).cloned().unwrap_or_default();
            path.insert(0, (field, class));
            alias.clone_from(parent);
        }
        Self::get_association_path_value(em, &Value::Object(entity.clone()), &path)
    }

    fn get_association_path_value(
        em: &EntityManager,
        value: &Value,
        path: &[(String, String)],
    ) -> Result<Option<Value>, Thrown> {
        let Some(((field_name, class), rest)) = path.split_first() else {
            return Ok(None);
        };
        let metadata = em.get_class_metadata(class)?;
        let (is_to_one, value) = {
            let info = metadata.read();
            let is_to_one = info
                .association_mappings
                .get(field_name)
                .is_some_and(crate::mapping::AssociationMapping::is_to_one);
            let Value::Object(object) = value else {
                return Ok(None);
            };
            (is_to_one, info.get_field_value(object, field_name)?)
        };
        if value.is_null() {
            return Ok(None);
        }
        if rest.is_empty() {
            return Ok(Some(value));
        }
        // Handle *-to-one associations
        if is_to_one {
            return Self::get_association_path_value(em, &value, rest);
        }
        let mut values = Vec::new();
        for item in elements(&value).values() {
            values.push(Self::get_association_path_value(em, item, rest)?.unwrap_or(Value::Null));
        }
        Ok(Some(Value::Array(PhpArray::list(values))))
    }
}

impl QueryCache for DefaultQueryCache {
    fn clear(&self) -> Result<bool, Thrown> {
        self.region.evict_all()
    }

    fn put<'a>(
        &'a self,
        key: &'a QueryCacheKey,
        rsm: &'a ResultSetMapping,
        result: &'a PhpArray,
        hints: &'a PhpArray,
    ) -> Fut<'a, bool> {
        Box::pin(std::future::ready(self.do_put(key, rsm, result, hints)))
    }

    fn get<'a>(
        &'a self,
        key: &'a QueryCacheKey,
        rsm: &'a ResultSetMapping,
        _hints: &'a PhpArray,
    ) -> Fut<'a, Option<PhpArray>> {
        Box::pin(self.do_get(key, rsm))
    }

    fn get_region(&self) -> Arc<dyn Region> {
        Arc::clone(&self.region)
    }
}
