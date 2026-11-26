//! A method invoker based on entity lifecycle.
//!
//! Upstream: doctrine/orm src/Event/ListenersInvoker.php

use std::sync::Arc;

use php_value::{Object, Thrown};
use php2rust_doctrine_event_manager::{
    EventArgs, EventDispatcher, EventListenerIntrospector, EventManager,
};

use crate::mapping::ClassMetadataInfo;
use crate::mapping::entity_listener_resolver::EntityListenerResolver;
use crate::object::{call_listener_method, call_method};

/// `Doctrine\ORM\Event\ListenersInvoker`.
#[derive(Debug)]
pub struct ListenersInvoker {
    /// The Entity listener resolver.
    resolver: Arc<dyn EntityListenerResolver>,
}

impl ListenersInvoker {
    pub const INVOKE_NONE: i32 = 0;
    pub const INVOKE_LISTENERS: i32 = 1;
    pub const INVOKE_CALLBACKS: i32 = 2;
    pub const INVOKE_MANAGER: i32 = 4;

    /// `new ListenersInvoker($em)`: the resolver of the entity manager's
    /// configuration.
    pub fn new(resolver: Arc<dyn EntityListenerResolver>) -> Self {
        Self { resolver }
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
                let instance = self.resolver.resolve(&listener.class)?;
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
