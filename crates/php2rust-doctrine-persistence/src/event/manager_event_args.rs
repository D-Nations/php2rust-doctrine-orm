//! Provides event arguments for the preFlush event.
//!
//! Upstream: doctrine/persistence src/Event/ManagerEventArgs.php

use std::sync::Arc;

use php2rust_doctrine_event_manager::EventArgs;

use crate::ObjectManager;

/// `Doctrine\Persistence\Event\ManagerEventArgs`.
pub struct ManagerEventArgs {
    object_manager: Arc<dyn ObjectManager>,
}

impl ManagerEventArgs {
    pub fn new(object_manager: Arc<dyn ObjectManager>) -> Self {
        Self { object_manager }
    }

    /// Retrieves the associated `ObjectManager`.
    pub fn get_object_manager(&self) -> &Arc<dyn ObjectManager> {
        &self.object_manager
    }
}

impl EventArgs for ManagerEventArgs {}
