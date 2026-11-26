//! Class that holds event arguments for a preUpdate event.
//!
//! The change set is passed by reference in PHP: here it is a
//! [`Handle`] the caller keeps, and `setNewValue()` changes it for both.
//!
//! Upstream: doctrine/persistence src/Event/PreUpdateEventArgs.php

use std::sync::Arc;

use php_value::{Handle, Key, Object, PhpArray, Thrown, Value};
use php2rust_doctrine_event_manager::EventArgs;

use crate::ObjectManager;
use crate::event::LifecycleEventArgs;

/// `Doctrine\Persistence\Event\PreUpdateEventArgs`.
pub struct PreUpdateEventArgs {
    lifecycle: LifecycleEventArgs,
    /// Field name → `[old value, new value]`.
    entity_change_set: Handle<PhpArray>,
}

impl PreUpdateEventArgs {
    pub fn new(
        entity: Object,
        object_manager: Arc<dyn ObjectManager>,
        change_set: Handle<PhpArray>,
    ) -> Self {
        Self {
            lifecycle: LifecycleEventArgs::new(entity, object_manager),
            entity_change_set: change_set,
        }
    }

    /// The parent class's arguments.
    pub fn lifecycle(&self) -> &LifecycleEventArgs {
        &self.lifecycle
    }

    /// Retrieves the associated object.
    pub fn get_object(&self) -> &Object {
        self.lifecycle.get_object()
    }

    /// Retrieves the associated `ObjectManager`.
    pub fn get_object_manager(&self) -> &Arc<dyn ObjectManager> {
        self.lifecycle.get_object_manager()
    }

    /// Retrieves the entity changeset.
    pub fn get_entity_change_set(&self) -> PhpArray {
        self.entity_change_set.get()
    }

    /// Checks if field has a changeset.
    pub fn has_changed_field(&self, field: &str) -> bool {
        self.entity_change_set
            .with(|change_set| is_set(change_set.get(&Key::from(field))))
    }

    /// Gets the old value of the changeset of the changed field.
    pub fn get_old_value(&self, field: &str) -> Result<Value, Thrown> {
        self.assert_valid_field(field)?;
        Ok(self.change(field, 0))
    }

    /// Gets the new value of the changeset of the changed field.
    pub fn get_new_value(&self, field: &str) -> Result<Value, Thrown> {
        self.assert_valid_field(field)?;
        Ok(self.change(field, 1))
    }

    /// Sets the new value of this field.
    pub fn set_new_value(&self, field: &str, value: Value) -> Result<(), Thrown> {
        self.assert_valid_field(field)?;
        self.entity_change_set.with_mut(|change_set| {
            if let Some(Value::Array(change)) = change_set.get_mut(&Key::from(field)) {
                change.insert(1, value);
            }
        });
        Ok(())
    }

    /// `$this->entityChangeSet[$field][$index]`.
    fn change(&self, field: &str, index: i64) -> Value {
        self.entity_change_set.with(|change_set| {
            change_set
                .get(&Key::from(field))
                .and_then(Value::as_array)
                .and_then(|change| change.get(&Key::from(index)))
                .cloned()
                .unwrap_or_default()
        })
    }

    /// Asserts the field exists in changeset.
    fn assert_valid_field(&self, field: &str) -> Result<(), Thrown> {
        if !self.has_changed_field(field) {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!(
                    "Field \"{field}\" is not a valid field of the entity \"{}\" in PreUpdateEventArgs.",
                    self.get_object().class()
                ),
                0,
            ));
        }
        Ok(())
    }
}

/// `isset()`: present and not `null`.
fn is_set(value: Option<&Value>) -> bool {
    value.is_some_and(|v| !v.is_null())
}

impl EventArgs for PreUpdateEventArgs {}
