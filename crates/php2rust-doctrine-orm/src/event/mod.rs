//! `Doctrine\ORM\Event`: the event arguments the entity manager and the
//! unit of work dispatch, and the listeners invoker.
//!
//! The lifecycle event arguments (`PrePersistEventArgs`,
//! `PostPersistEventArgs`, `PreRemoveEventArgs`, `PostRemoveEventArgs`,
//! `PostUpdateEventArgs`, `PostLoadEventArgs`) are one type here that
//! carries the class name; `PreUpdateEventArgs` has its own type. The
//! manager events (`PreFlushEventArgs`, `OnFlushEventArgs`,
//! `PostFlushEventArgs`, `OnClearEventArgs`) are one type too. They hold
//! the entity manager (persistence's event arguments hold a synchronous
//! `ObjectManager`, which the asynchronous entity manager is not).
//!
//! Upstream: doctrine/orm src/Event/PrePersistEventArgs.php src/Event/PostPersistEventArgs.php src/Event/PreRemoveEventArgs.php
//! Upstream: doctrine/orm src/Event/PostRemoveEventArgs.php src/Event/PostUpdateEventArgs.php src/Event/PostLoadEventArgs.php
//! Upstream: doctrine/orm src/Event/PreUpdateEventArgs.php src/Event/PreFlushEventArgs.php src/Event/OnFlushEventArgs.php
//! Upstream: doctrine/orm src/Event/PostFlushEventArgs.php src/Event/OnClearEventArgs.php

mod listeners_invoker;

pub use listeners_invoker::ListenersInvoker;

use std::sync::Arc;

use php_value::{Handle, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_event_manager::EventArgs;

use crate::entity_manager::EntityManager;

pub const PRE_PERSIST_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PrePersistEventArgs";
pub const POST_PERSIST_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PostPersistEventArgs";
pub const PRE_REMOVE_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PreRemoveEventArgs";
pub const POST_REMOVE_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PostRemoveEventArgs";
pub const POST_UPDATE_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PostUpdateEventArgs";
pub const POST_LOAD_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PostLoadEventArgs";
pub const PRE_UPDATE_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PreUpdateEventArgs";
pub const PRE_FLUSH_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PreFlushEventArgs";
pub const ON_FLUSH_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\OnFlushEventArgs";
pub const POST_FLUSH_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\PostFlushEventArgs";
pub const ON_CLEAR_EVENT_ARGS: &str = "Doctrine\\ORM\\Event\\OnClearEventArgs";

/// A lifecycle event's arguments: the entity and the entity manager.
pub struct LifecycleEventArgs {
    class: &'static str,
    object: Object,
    object_manager: Arc<EntityManager>,
}

impl std::fmt::Debug for LifecycleEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecycleEventArgs")
            .field("class", &self.class)
            .finish_non_exhaustive()
    }
}

impl LifecycleEventArgs {
    /// `new $class($object, $objectManager)`.
    pub fn new(class: &'static str, object: Object, object_manager: Arc<EntityManager>) -> Self {
        Self {
            class,
            object,
            object_manager,
        }
    }

    /// The class of the event arguments (`PrePersistEventArgs`, ...).
    pub fn class(&self) -> &'static str {
        self.class
    }

    /// `getObject()`.
    pub fn get_object(&self) -> &Object {
        &self.object
    }

    /// `getObjectManager()`.
    pub fn get_object_manager(&self) -> &Arc<EntityManager> {
        &self.object_manager
    }
}

impl EventArgs for LifecycleEventArgs {}

/// `PreUpdateEventArgs`: the entity's change set, which a listener may
/// change (`setNewValue()`).
pub struct PreUpdateEventArgs {
    object: Object,
    object_manager: Arc<EntityManager>,
    entity_change_set: Handle<PhpArray>,
}

impl std::fmt::Debug for PreUpdateEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreUpdateEventArgs").finish_non_exhaustive()
    }
}

impl PreUpdateEventArgs {
    pub fn new(entity: Object, em: Arc<EntityManager>, change_set: Handle<PhpArray>) -> Self {
        Self {
            object: entity,
            object_manager: em,
            entity_change_set: change_set,
        }
    }

    pub fn get_object(&self) -> &Object {
        &self.object
    }

    pub fn get_object_manager(&self) -> &Arc<EntityManager> {
        &self.object_manager
    }

    /// Retrieves entity changeset.
    pub fn get_entity_change_set(&self) -> PhpArray {
        self.entity_change_set.get()
    }

    /// Checks if field has a changeset.
    pub fn has_changed_field(&self, field: &str) -> bool {
        self.entity_change_set
            .with(|set| set.contains_key(&field.into()))
    }

    fn assert_valid_field(&self, field: &str) -> Result<(), Thrown> {
        if self.has_changed_field(field) {
            return Ok(());
        }
        Err(Thrown::new(
            "InvalidArgumentException",
            format!(
                "Field \"{field}\" is not a valid field of the entity \"{}\" in PreUpdateEventArgs.",
                self.object.class()
            ),
            0,
        ))
    }

    fn change(&self, field: &str, index: i64) -> Result<Value, Thrown> {
        self.assert_valid_field(field)?;
        Ok(self
            .entity_change_set
            .with(|set| match set.get(&field.into()) {
                Some(Value::Array(pair)) => pair.get(&index.into()).cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            }))
    }

    /// Gets the old value of the changeset of the changed field.
    pub fn get_old_value(&self, field: &str) -> Result<Value, Thrown> {
        self.change(field, 0)
    }

    /// Gets the new value of the changeset of the changed field.
    pub fn get_new_value(&self, field: &str) -> Result<Value, Thrown> {
        self.change(field, 1)
    }

    /// Sets the new value of this field.
    pub fn set_new_value(&self, field: &str, value: Value) -> Result<(), Thrown> {
        self.assert_valid_field(field)?;
        self.entity_change_set.with_mut(|set| {
            if let Some(Value::Array(pair)) = set.get_mut(&field.into()) {
                pair.insert(1, value);
            }
        });
        Ok(())
    }
}

impl EventArgs for PreUpdateEventArgs {}

/// The arguments of the manager events (`preFlush`, `onFlush`,
/// `postFlush`, `onClear`).
pub struct ManagerEventArgs {
    class: &'static str,
    object_manager: Arc<EntityManager>,
}

impl std::fmt::Debug for ManagerEventArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagerEventArgs")
            .field("class", &self.class)
            .finish_non_exhaustive()
    }
}

impl ManagerEventArgs {
    pub fn new(class: &'static str, object_manager: Arc<EntityManager>) -> Self {
        Self {
            class,
            object_manager,
        }
    }

    pub fn class(&self) -> &'static str {
        self.class
    }

    /// `getObjectManager()`.
    pub fn get_object_manager(&self) -> &Arc<EntityManager> {
        &self.object_manager
    }
}

impl EventArgs for ManagerEventArgs {}

/// What a listener threw, as a `Thrown`.
pub fn listener_error(e: php2rust_doctrine_event_manager::ListenerError) -> Thrown {
    match e.downcast::<Thrown>() {
        Ok(thrown) => *thrown,
        Err(other) => Thrown::new("Exception", other.to_string(), 0),
    }
}

/// The class of ORM event arguments (`$eventArgs::class`): `None` for
/// arguments the ORM does not define.
pub fn event_args_class(args: &dyn EventArgs) -> Option<&'static str> {
    let any: &dyn std::any::Any = args;
    if let Some(args) = any.downcast_ref::<LifecycleEventArgs>() {
        return Some(args.class());
    }
    if any.downcast_ref::<PreUpdateEventArgs>().is_some() {
        return Some(PRE_UPDATE_EVENT_ARGS);
    }
    any.downcast_ref::<ManagerEventArgs>()
        .map(ManagerEventArgs::class)
}
