//! Contract for a Doctrine persistence layer `ClassMetadata` class to
//! implement.
//!
//! Upstream: doctrine/persistence src/Mapping/ClassMetadataFactory.php

use std::sync::Arc;

use php_value::Thrown;

use crate::mapping::ClassMetadata;

/// `Doctrine\Persistence\Mapping\ClassMetadataFactory`. Every
/// [`AbstractClassMetadataFactory`](crate::mapping::AbstractClassMetadataFactory)
/// is one.
pub trait ClassMetadataFactory: Send + Sync {
    /// Forces the factory to load the metadata of all classes known to the
    /// underlying mapping driver.
    fn get_all_metadata(&self) -> Result<Vec<Arc<dyn ClassMetadata>>, Thrown>;

    /// Gets the class metadata descriptor for a class.
    fn get_metadata_for(&self, class_name: &str) -> Result<Arc<dyn ClassMetadata>, Thrown>;

    /// Checks whether the factory has the metadata for a class loaded
    /// already.
    fn has_metadata_for(&self, class_name: &str) -> bool;

    /// Sets the metadata descriptor for a specific class.
    ///
    /// Deprecated since 4.2, use a custom `ClassMetadataFactory`
    /// implementation if you need to set metadata manually. A factory
    /// that holds one Rust type of metadata throws a `TypeError` for
    /// another type.
    fn set_metadata_for(
        &self,
        class_name: &str,
        class: Arc<dyn ClassMetadata>,
    ) -> Result<(), Thrown>;

    /// Returns whether the class with the specified name should have its
    /// metadata loaded. This is only the case if it is either mapped
    /// directly or as a `MappedSuperclass`.
    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown>;
}
