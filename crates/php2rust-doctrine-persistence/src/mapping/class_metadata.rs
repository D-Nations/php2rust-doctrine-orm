//! Contract for a Doctrine persistence layer `ClassMetadata` class to
//! implement.
//!
//! An implementation is a PHP object: the factory, the drivers and the
//! listeners share it (`Arc`), so a mapping driver that fills it in does
//! so through `&self` and downcasts it to its own class
//! (`(metadata as &dyn Any).downcast_ref::<T>()`, or
//! [`ClassMetadata::downcast_ref`]).
//!
//! Upstream: doctrine/persistence src/Mapping/ClassMetadata.php

use std::any::Any;

use php_value::{Object, PhpArray, Thrown};

use crate::php_reflection::ReflectionClass;

/// `Doctrine\Persistence\Mapping\ClassMetadata`.
pub trait ClassMetadata: Any + Send + Sync {
    /// Gets the fully-qualified class name of this persistent class.
    fn get_name(&self) -> String;

    /// Gets the mapped identifier field name.
    ///
    /// The returned structure is an array of the identifier field names.
    fn get_identifier(&self) -> Vec<String>;

    /// Gets the `ReflectionClass` instance for this mapped class.
    fn get_reflection_class(&self) -> Result<ReflectionClass, Thrown>;

    /// Checks if the given field name is a mapped identifier for this
    /// class.
    fn is_identifier(&self, field_name: &str) -> bool;

    /// Checks if the given field is a mapped property for this class.
    fn has_field(&self, field_name: &str) -> bool;

    /// Checks if the given field is a mapped association for this class.
    fn has_association(&self, field_name: &str) -> bool;

    /// Checks if the given field is a mapped single valued association for
    /// this class.
    fn is_single_valued_association(&self, field_name: &str) -> bool;

    /// Checks if the given field is a mapped collection valued association
    /// for this class.
    fn is_collection_valued_association(&self, field_name: &str) -> bool;

    /// A numerically indexed list of field names of this persistent class.
    ///
    /// This array includes identifier fields if present on this class.
    fn get_field_names(&self) -> Vec<String>;

    /// Returns an array of identifier field names numerically indexed.
    fn get_identifier_field_names(&self) -> Vec<String>;

    /// Returns a numerically indexed list of association names of this
    /// persistent class.
    ///
    /// This array includes identifier associations if present on this
    /// class.
    fn get_association_names(&self) -> Vec<String>;

    /// Returns a type name of this field.
    ///
    /// This type names can be implementation specific but should at least
    /// include the php types: integer, string, boolean, float/double,
    /// datetime.
    fn get_type_of_field(&self, field_name: &str) -> Result<Option<String>, Thrown>;

    /// Returns the target class name of the given association.
    fn get_association_target_class(&self, assoc_name: &str) -> Result<Option<String>, Thrown>;

    /// Checks if the association is the inverse side of a bidirectional
    /// association.
    fn is_association_inverse_side(&self, assoc_name: &str) -> bool;

    /// Returns the target field of the owning side of the association.
    fn get_association_mapped_by_target_field(&self, assoc_name: &str) -> Result<String, Thrown>;

    /// Returns the identifier of this object as an array with field name
    /// as key.
    ///
    /// Has to return an empty array if no identifier isset.
    fn get_identifier_values(&self, object: &Object) -> Result<PhpArray, Thrown>;
}

impl dyn ClassMetadata {
    /// `$metadata instanceof T`.
    pub fn downcast_ref<T: ClassMetadata>(&self) -> Option<&T> {
        (self as &dyn Any).downcast_ref()
    }
}
