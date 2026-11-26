//! PHP Runtime Reflection Service.
//!
//! Upstream: doctrine/persistence src/Mapping/RuntimeReflectionService.php

use std::sync::Arc;

use php_value::{Key, PhpArray, PhpObject, Thrown};

use crate::mapping::{ReflectionService, mapping_exception};
use crate::php_reflection::{self, PropertyAccessor, ReflectionClass};
use crate::reflection::{RuntimeReflectionProperty, TypedNoDefaultReflectionProperty};

/// `Doctrine\Persistence\Mapping\RuntimeReflectionService`.
#[derive(Debug, Default, Clone, Copy)]
pub struct RuntimeReflectionService;

impl RuntimeReflectionService {
    pub const CLASS: &str = "Doctrine\\Persistence\\Mapping\\RuntimeReflectionService";

    pub fn new() -> Self {
        Self
    }

    /// `getAccessibleProperty()` with its class: a
    /// `TypedNoDefaultReflectionProperty` for a property without a default
    /// value, else a `RuntimeReflectionProperty`.
    pub fn accessible_property(
        class: &str,
        property: &str,
    ) -> Result<Arc<dyn PropertyAccessor>, Thrown> {
        let reflection_property = RuntimeReflectionProperty::new(class, property)?;
        if !ReflectionClass::new(class)?
            .get_default_properties()
            .contains_key(&Key::from(property))
        {
            return Ok(Arc::new(TypedNoDefaultReflectionProperty::from(
                reflection_property,
            )));
        }
        Ok(Arc::new(reflection_property))
    }
}

impl ReflectionService for RuntimeReflectionService {
    fn get_parent_classes(&self, class: &str) -> Result<Vec<String>, Thrown> {
        if !php_reflection::class_exists(class) {
            return Err(mapping_exception::non_existing_class(class));
        }
        php_reflection::class_parents(class)
            .ok_or_else(|| mapping_exception::non_existing_class(class))
    }

    fn get_class_short_name(&self, class: &str) -> Result<String, Thrown> {
        Ok(ReflectionClass::new(class)?.get_short_name().to_owned())
    }

    fn get_class_namespace(&self, class: &str) -> Result<String, Thrown> {
        Ok(ReflectionClass::new(class)?.get_namespace_name().to_owned())
    }

    fn get_class(&self, class: &str) -> Result<ReflectionClass, Thrown> {
        ReflectionClass::new(class)
    }

    fn get_accessible_property(
        &self,
        class: &str,
        property: &str,
    ) -> Result<Option<Arc<dyn PropertyAccessor>>, Thrown> {
        Self::accessible_property(class, property).map(Some)
    }

    fn has_public_method(&self, class: &str, method: &str) -> bool {
        let Ok(reflection_class) = ReflectionClass::new(class) else {
            return false;
        };
        reflection_class
            .get_method(method)
            .is_ok_and(|m| m.is_public())
    }
}

/// The service is an object a property can hold.
impl PhpObject for RuntimeReflectionService {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn instance_of(&self, class: &str) -> bool {
        php_reflection::is_a(Self::CLASS, class)
    }
}
