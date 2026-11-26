//! Very simple reflection service abstraction.
//!
//! This is required inside metadata layers that may require either static
//! or runtime reflection.
//!
//! Upstream: doctrine/persistence src/Mapping/ReflectionService.php

use std::sync::Arc;

use php_value::Thrown;

use crate::php_reflection::{PropertyAccessor, ReflectionClass};

/// `Doctrine\Persistence\Mapping\ReflectionService`.
pub trait ReflectionService: Send + Sync {
    /// Returns an array of the parent classes (not interfaces) for the
    /// given class, nearest first.
    fn get_parent_classes(&self, class: &str) -> Result<Vec<String>, Thrown>;

    /// Returns the shortname of a class.
    fn get_class_short_name(&self, class: &str) -> Result<String, Thrown>;

    fn get_class_namespace(&self, class: &str) -> Result<String, Thrown>;

    /// Returns a reflection class instance or null.
    fn get_class(&self, class: &str) -> Result<ReflectionClass, Thrown>;

    /// Returns an accessible property or null.
    fn get_accessible_property(
        &self,
        class: &str,
        property: &str,
    ) -> Result<Option<Arc<dyn PropertyAccessor>>, Thrown>;

    /// Checks if the class have a public method with the given name.
    fn has_public_method(&self, class: &str, method: &str) -> bool;
}
