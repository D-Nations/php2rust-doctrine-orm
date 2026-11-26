//! Class that holds event arguments for a loadMetadata event.
//!
//! Upstream: doctrine/persistence src/Event/LoadClassMetadataEventArgs.php

use std::sync::Arc;

use php2rust_doctrine_event_manager::EventArgs;

use crate::ObjectManager;
use crate::mapping::ClassMetadata;

/// `Doctrine\Persistence\Event\LoadClassMetadataEventArgs`.
pub struct LoadClassMetadataEventArgs {
    class_metadata: Arc<dyn ClassMetadata>,
    object_manager: Arc<dyn ObjectManager>,
}

impl LoadClassMetadataEventArgs {
    pub fn new(
        class_metadata: Arc<dyn ClassMetadata>,
        object_manager: Arc<dyn ObjectManager>,
    ) -> Self {
        Self {
            class_metadata,
            object_manager,
        }
    }

    /// Retrieves the associated `ClassMetadata`.
    pub fn get_class_metadata(&self) -> &Arc<dyn ClassMetadata> {
        &self.class_metadata
    }

    /// Retrieves the associated `ObjectManager`.
    pub fn get_object_manager(&self) -> &Arc<dyn ObjectManager> {
        &self.object_manager
    }
}

impl EventArgs for LoadClassMetadataEventArgs {}
