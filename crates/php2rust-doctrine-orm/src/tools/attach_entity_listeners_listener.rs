//! Mechanism to programmatically attach entity listeners.
//!
//! The metadata factory's `loadClassMetadata` event is a hook of its
//! configuration: [`AttachEntityListenersListener::hook`] is the listener
//! that `addEventListener(Events::loadClassMetadata, ...)` registers.
//!
//! Upstream: doctrine/orm src/Tools/AttachEntityListenersListener.php

use std::sync::{Arc, Mutex, PoisonError};

use php_value::Thrown;

use crate::mapping::ClassMetadataInfo;
use crate::mapping::builder::EntityListenerBuilder;
use crate::mapping::class_metadata_factory::LoadClassMetadataHook;

/// An entity listener to attach.
#[derive(Debug, Clone)]
struct Listener {
    event: Option<String>,
    class: String,
    method: Option<String>,
}

/// `Doctrine\ORM\Tools\AttachEntityListenersListener`.
#[derive(Debug, Default)]
pub struct AttachEntityListenersListener {
    entity_listeners: Mutex<Vec<(String, Vec<Listener>)>>,
}

impl AttachEntityListenersListener {
    pub fn new() -> Self {
        Self::default()
    }

    fn listeners(&self) -> std::sync::MutexGuard<'_, Vec<(String, Vec<Listener>)>> {
        self.entity_listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds an entity listener for a specific entity.
    pub fn add_entity_listener(
        &self,
        entity_class: &str,
        listener_class: &str,
        event_name: Option<&str>,
        listener_callback: Option<&str>,
    ) {
        let entity_class = entity_class.trim_start_matches('\\').to_owned();
        let listener = Listener {
            event: event_name.map(str::to_owned),
            class: listener_class.to_owned(),
            method: listener_callback.or(event_name).map(str::to_owned),
        };
        let mut listeners = self.listeners();
        match listeners
            .iter_mut()
            .find(|(class, _)| *class == entity_class)
        {
            Some((_, entries)) => entries.push(listener),
            None => listeners.push((entity_class, vec![listener])),
        }
    }

    /// Processes event and attach the entity listener.
    pub fn load_class_metadata(&self, metadata: &mut ClassMetadataInfo) -> Result<(), Thrown> {
        let Some(listeners) = self
            .listeners()
            .iter()
            .find(|(class, _)| *class == metadata.name)
            .map(|(_, listeners)| listeners.clone())
        else {
            return Ok(());
        };

        for listener in listeners {
            match (&listener.event, &listener.method) {
                (Some(event), Some(method)) => {
                    metadata.add_entity_listener(event, &listener.class, method)?;
                }
                _ => EntityListenerBuilder::bind_entity_listener(metadata, &listener.class)?,
            }
        }
        Ok(())
    }

    /// The `loadClassMetadata` listener.
    pub fn hook(self: &Arc<Self>) -> LoadClassMetadataHook {
        let listener = Arc::clone(self);
        Arc::new(move |class| listener.load_class_metadata(&mut class.write()))
    }
}
