//! Provides event arguments for the onClear event.
//!
//! Upstream: doctrine/persistence src/Event/OnClearEventArgs.php

use std::sync::Arc;

use php2rust_doctrine_event_manager::EventArgs;

use crate::ObjectManager;

/// `Doctrine\Persistence\Event\OnClearEventArgs`.
pub struct OnClearEventArgs {
    object_manager: Arc<dyn ObjectManager>,
}

impl OnClearEventArgs {
    pub fn new(object_manager: Arc<dyn ObjectManager>) -> Self {
        Self { object_manager }
    }

    /// Retrieves the associated `ObjectManager`.
    pub fn get_object_manager(&self) -> &Arc<dyn ObjectManager> {
        &self.object_manager
    }
}

impl EventArgs for OnClearEventArgs {}
