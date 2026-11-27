//! `ResolveTargetEntityListener`: mechanism to overwrite interfaces or
//! classes specified as association targets.
//!
//! The metadata factory's events are hooks of its configuration, so
//! `addEventSubscriber()` is [`ResolveTargetEntityListener::subscribe`]
//! (both events) and `addEventListener(Events::loadClassMetadata, ...)`
//! is [`ResolveTargetEntityListener::listen_to_load_class_metadata`], on
//! the entity manager's factory. `onClassMetadataNotFound`'s object
//! manager is that entity manager.
//!
//! Upstream: doctrine/orm src/Tools/ResolveTargetEntityListener.php

use std::sync::{Arc, Mutex, PoisonError};

use php_value::{Key, PhpArray, Thrown, Value};

use crate::entity_manager::EntityManager;
use crate::events;
use crate::mapping::association_mapping as am;
use crate::mapping::{AssociationMapping, ClassMetadata, ClassMetadataInfo};

/// `Doctrine\ORM\Tools\ResolveTargetEntityListener`.
#[derive(Debug, Default)]
pub struct ResolveTargetEntityListener {
    /// Indexed by original entity name.
    resolve_target_entities: Mutex<Vec<(String, PhpArray)>>,
}

/// `array_replace_recursive($array, $replacements)`.
fn array_replace_recursive(array: &PhpArray, replacements: &PhpArray) -> PhpArray {
    let mut result = array.clone();
    for (key, value) in replacements.iter() {
        let replaced = match (result.get(key), value) {
            (Some(Value::Array(existing)), Value::Array(replacement)) => {
                Value::Array(array_replace_recursive(existing, replacement))
            }
            _ => value.clone(),
        };
        result.insert(key.clone(), replaced);
    }
    result
}

impl ResolveTargetEntityListener {
    pub fn new() -> Self {
        Self::default()
    }

    fn entities(&self) -> std::sync::MutexGuard<'_, Vec<(String, PhpArray)>> {
        self.resolve_target_entities
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn resolved(&self, original_entity: &str) -> Option<PhpArray> {
        self.entities()
            .iter()
            .find(|(name, _)| name == original_entity)
            .map(|(_, mapping)| mapping.clone())
    }

    fn target_entity(mapping: &PhpArray) -> String {
        match mapping.get(&Key::from("targetEntity")) {
            Some(value) => value.to_php_string().map_or_else(String::new, |text| {
                String::from_utf8_lossy(text.as_bytes()).into_owned()
            }),
            None => String::new(),
        }
    }

    pub fn get_subscribed_events(&self) -> Vec<&'static str> {
        vec![
            events::LOAD_CLASS_METADATA,
            events::ON_CLASS_METADATA_NOT_FOUND,
        ]
    }

    /// Adds a target-entity class name to resolve to a new class name.
    pub fn add_resolve_target_entity(
        &self,
        original_entity: &str,
        new_entity: &str,
        mut mapping: PhpArray,
    ) {
        mapping.insert(
            Key::from("targetEntity"),
            Value::string(new_entity.trim_start_matches('\\')),
        );
        let original_entity = original_entity.trim_start_matches('\\').to_owned();
        let mut entities = self.entities();
        match entities
            .iter_mut()
            .find(|(name, _)| *name == original_entity)
        {
            Some(entry) => entry.1 = mapping,
            None => entities.push((original_entity, mapping)),
        }
    }

    /// `addEventSubscriber($listener)` on the entity manager's event
    /// manager: both events.
    pub fn subscribe(self: &Arc<Self>, em: &Arc<EntityManager>) -> Result<(), Thrown> {
        self.listen_to_load_class_metadata(em)?;
        let listener = Arc::clone(self);
        let object_manager = Arc::downgrade(em);
        em.get_metadata_factory()
            .add_class_metadata_not_found_hook(Arc::new(move |class_name| {
                let Some(em) = object_manager.upgrade() else {
                    return Ok(None);
                };
                listener
                    .on_class_metadata_not_found(class_name, |class| em.get_class_metadata(class))
            }))
    }

    /// `addEventListener(Events::loadClassMetadata, $listener)` on the
    /// entity manager's event manager.
    pub fn listen_to_load_class_metadata(
        self: &Arc<Self>,
        em: &Arc<EntityManager>,
    ) -> Result<(), Thrown> {
        let listener = Arc::clone(self);
        em.get_metadata_factory()
            .add_load_class_metadata_hook(Arc::new(move |class| {
                listener.load_class_metadata(&mut class.write())
            }))
    }

    /// The `onClassMetadataNotFound` event: the metadata of the class
    /// the missing class resolves to, from the object manager's
    /// `getClassMetadata()`.
    pub fn on_class_metadata_not_found(
        &self,
        class_name: &str,
        get_class_metadata: impl FnOnce(&str) -> Result<Arc<ClassMetadata>, Thrown>,
    ) -> Result<Option<Arc<ClassMetadata>>, Thrown> {
        match self.resolved(class_name) {
            Some(mapping) => get_class_metadata(&Self::target_entity(&mapping)).map(Some),
            None => Ok(None),
        }
    }

    /// Processes event and resolves new target entity names.
    pub fn load_class_metadata(&self, cm: &mut ClassMetadataInfo) -> Result<(), Thrown> {
        let mappings: Vec<AssociationMapping> = cm.association_mappings.values().cloned().collect();
        for mapping in mappings {
            if let Some(new_mapping) = self.resolved(&mapping.target_entity) {
                Self::remap_association(cm, &mapping, &new_mapping)?;
            }
        }

        let discriminator_map: Vec<(Key, Value)> = cm
            .discriminator_map
            .iter()
            .map(|(value, class)| (value.clone(), class.clone()))
            .collect();
        for (value, class) in discriminator_map {
            let class = class.to_php_string().map_or_else(String::new, |text| {
                String::from_utf8_lossy(text.as_bytes()).into_owned()
            });
            if let Some(mapping) = self.resolved(&class) {
                cm.add_discriminator_map_class(&value, &Self::target_entity(&mapping))?;
            }
        }
        Ok(())
    }

    fn remap_association(
        class_metadata: &mut ClassMetadataInfo,
        mapping: &AssociationMapping,
        resolved: &PhpArray,
    ) -> Result<(), Thrown> {
        let mut new_mapping = array_replace_recursive(&mapping.to_array()?, resolved);
        new_mapping.insert(
            Key::from("fieldName"),
            Value::string(mapping.field_name.as_str()),
        );

        class_metadata
            .association_mappings
            .remove(&mapping.field_name);

        match mapping.r#type()? {
            am::MANY_TO_MANY => class_metadata.map_many_to_many(new_mapping),
            am::MANY_TO_ONE => class_metadata.map_many_to_one(new_mapping),
            am::ONE_TO_MANY => class_metadata.map_one_to_many(new_mapping),
            am::ONE_TO_ONE => class_metadata.map_one_to_one(new_mapping),
            _ => Ok(()),
        }
    }
}
