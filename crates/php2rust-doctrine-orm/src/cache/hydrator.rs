//! The entity cache hydrator: the `EntityHydrator` interface and
//! `DefaultEntityHydrator`, which builds an entity's cache entry from
//! its original data and creates the entity back from an entry.
//!
//! The collection cache hydrator: the `CollectionHydrator` interface
//! and `DefaultCollectionHydrator`, which keeps a collection as the cache
//! keys of its elements; a collection is a PHP array of its elements.
//!
//! Upstream: doctrine/orm src/Cache/EntityHydrator.php src/Cache/DefaultEntityHydrator.php
//! Upstream: doctrine/orm src/Cache/CollectionHydrator.php src/Cache/DefaultCollectionHydrator.php

use std::fmt::Debug;
use std::sync::{Arc, Weak};

use php_value::{Key, Object, PhpArray, Thrown, Value};

use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadata;
use crate::mapping::class_metadata::FETCH_EAGER;
use crate::unit_of_work::{Fut, Hints};
use crate::utility::flatten_identifier;

use crate::persistent_collection::PersistentCollection;

use super::{
    AssociationCacheEntry, CacheEntry, CollectionCacheEntry, CollectionCacheKey, EntityCacheEntry,
    EntityCacheKey,
};

/// `Doctrine\ORM\Cache\EntityHydrator`: hydrator cache entry for
/// entities.
pub trait EntityHydrator: Debug + Send + Sync {
    /// Builds the cache entry of an entity.
    fn build_cache_entry(
        &self,
        metadata: &ClassMetadata,
        key: &EntityCacheKey,
        entity: &Object,
    ) -> Result<EntityCacheEntry, Thrown>;

    /// Creates the entity of a cache entry (or refreshes `entity` from
    /// it); `None` when an eagerly loaded association is not cached.
    fn load_cache_entry<'a>(
        &'a self,
        metadata: &'a ClassMetadata,
        key: &'a EntityCacheKey,
        entry: &'a EntityCacheEntry,
        entity: Option<&'a Object>,
    ) -> Fut<'a, Option<Object>>;
}

pub(crate) fn em_of(em: &Weak<EntityManager>) -> Result<Arc<EntityManager>, Thrown> {
    em.upgrade()
        .ok_or_else(|| Thrown::new("Error", "The EntityManager is gone.", 0))
}

/// `$data[$name]` when it is set (`isset()`).
fn isset<'a>(data: &'a PhpArray, name: &str) -> Option<&'a Value> {
    data.get(&Key::from(name)).filter(|value| !value.is_null())
}

/// `EntityCacheEntry::resolveAssociationEntries()`: the entity data with
/// each `AssociationCacheEntry` replaced by a reference.
pub async fn resolve_association_entries(
    entry: &EntityCacheEntry,
    em: &EntityManager,
) -> Result<PhpArray, Thrown> {
    let mut data = PhpArray::new();
    for (key, value) in &entry.data {
        let value = match AssociationCacheEntry::from_value(value) {
            Some(association) => em
                .get_reference(
                    &association.class,
                    Value::Array(association.identifier.clone()),
                )
                .await?
                .map_or(Value::Null, Value::Object),
            None => value.clone(),
        };
        data.insert(key.clone(), value);
    }
    Ok(data)
}

/// The hints the cache hydrators create entities with
/// (`Query::HINT_CACHE_ENABLED`).
pub(crate) fn cache_hints() -> Hints {
    Hints {
        cache_enabled: true,
        ..Hints::default()
    }
}

/// `Doctrine\ORM\Cache\DefaultEntityHydrator`: default hydrator cache
/// for entities.
#[derive(Debug)]
pub struct DefaultEntityHydrator {
    em: Weak<EntityManager>,
}

impl DefaultEntityHydrator {
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            em: Arc::downgrade(em),
        }
    }

    /// The identifier of an association that is not cached, as the
    /// owning side's join columns.
    fn put_association_ids(
        em: &EntityManager,
        data: &mut PhpArray,
        assoc: &crate::mapping::AssociationMapping,
        value: &Value,
    ) -> Result<(), Thrown> {
        let target_class = em.get_class_metadata(&assoc.target_entity)?;
        let owning_association = em.get_metadata_factory().get_owning_side(assoc)?;
        let association_ids = {
            let target = target_class.read();
            let identifier_values = match value {
                Value::Object(object) => target.get_identifier_values(object)?,
                _ => PhpArray::new(),
            };
            flatten_identifier(em, &target, &identifier_values)?
        };
        data.remove(&Key::from(assoc.field_name.as_str()));
        let target = target_class.read();
        for (field_name, field_value) in &association_ids {
            let field_name = crate::utility::key_text(field_name);
            if let Some(field_mapping) = target.field_mappings.get(&field_name) {
                if let Some(Value::String(local)) = owning_association
                    .target_to_source_key_columns
                    .get(&Key::from(field_mapping.column_name.as_str()))
                {
                    data.insert(Key::from(local.clone()), field_value.clone());
                }
                continue;
            }
            let Some(target_assoc) = target.association_mappings.get(&field_name) else {
                continue;
            };
            for (referenced_column, local_column) in &assoc.target_to_source_key_columns {
                if target_assoc
                    .source_to_target_key_columns
                    .contains_key(referenced_column)
                    && let Value::String(local) = local_column
                {
                    data.insert(Key::from(local.clone()), field_value.clone());
                }
            }
        }
        Ok(())
    }
}

impl EntityHydrator for DefaultEntityHydrator {
    fn build_cache_entry(
        &self,
        metadata: &ClassMetadata,
        _key: &EntityCacheKey,
        entity: &Object,
    ) -> Result<EntityCacheEntry, Thrown> {
        let em = em_of(&self.em)?;
        let uow = em.get_unit_of_work();
        let mut data = uow.get_original_entity_data(entity);
        let info = metadata.read();
        // why update has no identifier values ?
        for (key, value) in &info.get_identifier_values(entity)? {
            data.insert(key.clone(), value.clone());
        }

        if info.requires_fetch_after_change {
            if info.is_versioned
                && let Some(version_field) = &info.version_field
            {
                let version = info.get_field_value(entity, version_field)?;
                data.insert(version_field.as_str(), version);
            }
            for (name, field_mapping) in info.field_mappings.iter() {
                if field_mapping.generated.is_some() {
                    data.insert(name, info.get_field_value(entity, name)?);
                }
            }
        }

        for (name, assoc) in info.association_mappings.iter() {
            let Some(value) = isset(&data, name).cloned() else {
                continue;
            };

            if !assoc.is_to_one() {
                data.remove(&Key::from(name));
                continue;
            }

            if assoc.cache.is_none() {
                Self::put_association_ids(&em, &mut data, assoc, &value)?;
                continue;
            }

            if assoc.id != Some(true) {
                let Value::Object(target) = &value else {
                    continue;
                };
                let target_id = uow.get_entity_identifier(target)?;
                data.insert(
                    name,
                    AssociationCacheEntry::new(target.class(), target_id).into_value(),
                );
                continue;
            }

            // handle association identifier
            let target_id = match &value {
                Value::Object(target) if uow.is_in_identity_map(target)? => {
                    Value::Array(uow.get_entity_identifier(target)?)
                }
                other => other.clone(),
            };

            // @TODO - fix it !
            // handle UnitOfWork#createEntity hash generation
            let target_id = match target_id {
                Value::Array(target_id) => target_id,
                target_id => {
                    if let Some((_, Value::String(column))) = assoc.join_column_field_names.first()
                    {
                        data.insert(Key::from(column.clone()), target_id.clone());
                    }
                    let target_entity = em.get_class_metadata(&assoc.target_entity)?;
                    let mut id = PhpArray::new();
                    id.insert(
                        target_entity
                            .read()
                            .identifier
                            .first()
                            .cloned()
                            .unwrap_or_default()
                            .as_str(),
                        target_id,
                    );
                    id
                }
            };

            data.insert(
                name,
                AssociationCacheEntry::new(assoc.target_entity.as_str(), target_id).into_value(),
            );
        }

        Ok(EntityCacheEntry::new(info.name.as_str(), data))
    }

    fn load_cache_entry<'a>(
        &'a self,
        metadata: &'a ClassMetadata,
        key: &'a EntityCacheKey,
        entry: &'a EntityCacheEntry,
        entity: Option<&'a Object>,
    ) -> Fut<'a, Option<Object>> {
        Box::pin(async move {
            let em = em_of(&self.em)?;
            let uow = em.get_unit_of_work();
            let mut data = entry.data.clone();
            let mut hints = cache_hints();
            if let Some(entity) = entity {
                hints.refresh = true;
                hints.refresh_entity = Some(entity.clone());
            }

            let associations: Vec<_> = metadata
                .read()
                .association_mappings
                .iter()
                .map(|(name, assoc)| (name.to_owned(), assoc.clone()))
                .collect();
            for (name, assoc) in associations {
                if assoc.cache.is_none() {
                    continue;
                }
                let Some(value) = isset(&data, &name).cloned() else {
                    continue;
                };
                let Some(association) = AssociationCacheEntry::from_value(&value).cloned() else {
                    return Err(Thrown::new(
                        "Error",
                        format!(
                            "Attempt to read property \"class\" on {}",
                            value.debug_type()
                        ),
                        0,
                    ));
                };
                let is_eager_load = assoc.fetch == Some(FETCH_EAGER)
                    || (assoc.is_one_to_one() && !assoc.is_owning_side());

                if !is_eager_load {
                    let reference = em
                        .get_reference(&association.class, Value::Array(association.identifier))
                        .await?;
                    data.insert(name.as_str(), reference.map_or(Value::Null, Value::Object));
                    continue;
                }

                let assoc_metadata = em.get_class_metadata(&assoc.target_entity)?;
                let root = assoc_metadata.read().root_entity_name.clone();
                let assoc_key = EntityCacheKey::new(root, &association.identifier);
                let assoc_persister = uow
                    .get_cached_entity_persister(&assoc.target_entity)?
                    .ok_or_else(|| super::persister::not_cached(&assoc.target_entity))?;
                let assoc_region = assoc_persister.get_cache_region();
                let Some(CacheEntry::Entity(assoc_entry)) =
                    assoc_region.get(&assoc_key.clone().into())?
                else {
                    return Ok(None);
                };
                let assoc_data = resolve_association_entries(&assoc_entry, &em).await?;
                let created = uow
                    .create_entity(&assoc_entry.class, assoc_data, &mut hints)
                    .await?;
                data.insert(name.as_str(), Value::Object(created));
            }

            if let Some(entity) = entity {
                uow.register_managed(entity, key.identifier.clone(), data.clone())?;
            }

            let result = uow.create_entity(&entry.class, data, &mut hints).await?;
            uow.hydration_complete()?;
            Ok(Some(result))
        })
    }
}

/// `Doctrine\ORM\Cache\CollectionHydrator`: hydrator cache entry for
/// collections.
pub trait CollectionHydrator: Debug + Send + Sync {
    /// Builds the cache entry of a collection (`$collection` as an
    /// array of its elements).
    fn build_cache_entry(
        &self,
        metadata: &ClassMetadata,
        key: &CollectionCacheKey,
        collection: &PhpArray,
    ) -> Result<CollectionCacheEntry, Thrown>;

    /// Loads the elements of a cached collection into `collection`;
    /// `None` when one of them is not cached.
    fn load_cache_entry<'a>(
        &'a self,
        metadata: &'a ClassMetadata,
        key: &'a CollectionCacheKey,
        entry: &'a CollectionCacheEntry,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Option<PhpArray>>;
}

/// `Doctrine\ORM\Cache\DefaultCollectionHydrator`: default hydrator
/// cache for collections.
#[derive(Debug)]
pub struct DefaultCollectionHydrator {
    em: Weak<EntityManager>,
}

impl DefaultCollectionHydrator {
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            em: Arc::downgrade(em),
        }
    }
}

impl CollectionHydrator for DefaultCollectionHydrator {
    fn build_cache_entry(
        &self,
        metadata: &ClassMetadata,
        _key: &CollectionCacheKey,
        collection: &PhpArray,
    ) -> Result<CollectionCacheEntry, Thrown> {
        let em = em_of(&self.em)?;
        let uow = em.get_unit_of_work();
        let root = metadata.read().root_entity_name.clone();
        let mut data = Vec::with_capacity(collection.len());
        for (index, entity) in collection {
            let Value::Object(entity) = entity else {
                return Err(Thrown::new(
                    "TypeError",
                    format!(
                        "Doctrine\\ORM\\UnitOfWork::getEntityIdentifier(): Argument #1 ($entity) must be of type object, {} given",
                        entity.debug_type()
                    ),
                    0,
                ));
            };
            let identifier = uow.get_entity_identifier(entity)?;
            data.push((
                index.clone(),
                EntityCacheKey::new(root.as_str(), &identifier).into(),
            ));
        }
        Ok(CollectionCacheEntry::new(data))
    }

    fn load_cache_entry<'a>(
        &'a self,
        metadata: &'a ClassMetadata,
        key: &'a CollectionCacheKey,
        entry: &'a CollectionCacheEntry,
        collection: &'a Arc<PersistentCollection>,
    ) -> Fut<'a, Option<PhpArray>> {
        Box::pin(async move {
            let em = em_of(&self.em)?;
            let uow = em.get_unit_of_work();
            let assoc = metadata
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
            let target_persister = uow
                .get_cached_entity_persister(&assoc.target_entity)?
                .ok_or_else(|| super::persister::not_cached(&assoc.target_entity))?;
            let target_region = target_persister.get_cache_region();

            let Some(entity_entries) = target_region.get_multiple(entry)? else {
                return Ok(None);
            };

            let mut list = PhpArray::new();
            for (index, entity_entry) in entity_entries {
                let CacheEntry::Entity(entity_entry) = entity_entry else {
                    return Err(Thrown::new(
                        "Error",
                        "Call to undefined method resolveAssociationEntries()",
                        0,
                    ));
                };
                let data = resolve_association_entries(&entity_entry, &em).await?;
                let mut hints = cache_hints();
                let entity = uow
                    .create_entity(&entity_entry.class, data, &mut hints)
                    .await?;
                collection.hydrate_set(index.clone(), Value::Object(entity.clone()))?;
                list.insert(index, Value::Object(entity));
            }

            uow.hydration_complete()?;

            Ok(Some(list))
        })
    }
}
