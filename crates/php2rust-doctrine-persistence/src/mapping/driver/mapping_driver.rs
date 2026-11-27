//! Contract for metadata drivers.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/MappingDriver.php

use std::any::Any;

use php_value::Thrown;

use crate::mapping::ClassMetadata;

/// `Doctrine\Persistence\Mapping\Driver\MappingDriver`.
pub trait MappingDriver: Any + Send + Sync {
    /// Loads the metadata for the specified class into the provided
    /// container (which the driver downcasts to its metadata class).
    fn load_metadata_for_class(
        &self,
        class_name: &str,
        metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown>;

    /// Gets the names of all mapped classes known to this driver.
    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown>;

    /// Returns whether the class with the specified name should have its
    /// metadata loaded. This is only the case if it is either mapped as an
    /// Entity or a `MappedSuperclass`.
    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown>;
}

impl dyn MappingDriver {
    /// `$driver instanceof T`.
    pub fn downcast_ref<T: MappingDriver>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}
