//! Lifecycle Events are triggered by the `UnitOfWork` during lifecycle
//! transitions of entities.
//!
//! Upstream: doctrine/persistence src/Event/LifecycleEventArgs.php

use std::sync::Arc;

use php_value::Object;
use php2rust_doctrine_event_manager::EventArgs;

use crate::ObjectManager;

/// `Doctrine\Persistence\Event\LifecycleEventArgs`.
pub struct LifecycleEventArgs {
    object: Object,
    object_manager: Arc<dyn ObjectManager>,
}

impl LifecycleEventArgs {
    pub fn new(object: Object, object_manager: Arc<dyn ObjectManager>) -> Self {
        Self {
            object,
            object_manager,
        }
    }

    /// Retrieves the associated object.
    pub fn get_object(&self) -> &Object {
        &self.object
    }

    /// Retrieves the associated `ObjectManager`.
    pub fn get_object_manager(&self) -> &Arc<dyn ObjectManager> {
        &self.object_manager
    }
}

impl EventArgs for LifecycleEventArgs {}
