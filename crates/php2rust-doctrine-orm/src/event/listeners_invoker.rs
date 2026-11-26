//! A method invoker based on entity lifecycle.
//!
//! The entity listener resolver is `DefaultEntityListenerResolver`'s:
//! one instance of each listener class, created without arguments
//! (`new $className()`; the instance has its properties' defaults).
//!
//! Upstream: doctrine/orm src/Event/ListenersInvoker.php src/Mapping/DefaultEntityListenerResolver.php

use std::sync::{Mutex, PoisonError};

use php_value::{Object, Thrown};
use php2rust_doctrine_event_manager::{
    EventArgs, EventDispatcher, EventListenerIntrospector, EventManager,
};

use crate::mapping::ClassMetadataInfo;
use crate::object::{EntityObject, call_listener_method, call_method};

/// `Doctrine\ORM\Event\ListenersInvoker`.
#[derive(Debug, Default)]
pub struct ListenersInvoker {
    /// The resolver's listener instances, by class.
    instances: Mutex<Vec<(String, Object)>>,
}

impl ListenersInvoker {
    pub const INVOKE_NONE: i32 = 0;
    pub const INVOKE_LISTENERS: i32 = 1;
    pub const INVOKE_CALLBACKS: i32 = 2;
    pub const INVOKE_MANAGER: i32 = 4;

    pub fn new() -> Self {
        Self::default()
    }

    /// Get the subscribed event systems.
    pub fn get_subscribed_systems(
        metadata: &ClassMetadataInfo,
        event_manager: &EventManager,
        event_name: &str,
    ) -> i32 {
        let mut invoke = Self::INVOKE_NONE;
        if metadata.lifecycle_callbacks.contains_key(event_name) {
            invoke |= Self::INVOKE_CALLBACKS;
        }
        if metadata.entity_listeners.contains_key(event_name) {
            invoke |= Self::INVOKE_LISTENERS;
        }
        if event_manager.has_listeners(event_name) {
            invoke |= Self::INVOKE_MANAGER;
        }
        invoke
    }

    fn resolve(&self, class: &str) -> Result<Object, Thrown> {
        let mut instances = self
            .instances
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some((_, instance)) = instances
            .iter()
            .find(|(c, _)| c.eq_ignore_ascii_case(class))
        {
            return Ok(instance.clone());
        }
        let instance = EntityObject::object(class)
            .ok_or_else(|| Thrown::new("Error", format!("Class \"{class}\" not found"), 0))?;
        instances.push((class.to_owned(), instance.clone()));
        Ok(instance)
    }

    /// Clears the resolver's instances (`DefaultEntityListenerResolver::clear()`).
    pub fn clear(&self) {
        self.instances
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    /// Dispatches the lifecycle event of the given entity.
    pub fn invoke(
        &self,
        metadata: &ClassMetadataInfo,
        event_manager: &EventManager,
        event_name: &str,
        entity: &Object,
        event: &dyn EventArgs,
        invoke: i32,
    ) -> Result<(), Thrown> {
        if invoke & Self::INVOKE_CALLBACKS != 0
            && let Some(callbacks) = metadata.lifecycle_callbacks.get(event_name)
        {
            for callback in callbacks {
                call_method(entity, callback, event)?;
            }
        }
        if invoke & Self::INVOKE_LISTENERS != 0
            && let Some(listeners) = metadata.entity_listeners.get(event_name)
        {
            for listener in listeners {
                let instance = self.resolve(&listener.class)?;
                call_listener_method(&instance, &listener.method, entity, event)?;
            }
        }
        if invoke & Self::INVOKE_MANAGER != 0 {
            event_manager
                .dispatch_event(event_name, Some(event))
                .map_err(super::listener_error)?;
        }
        Ok(())
    }
}
