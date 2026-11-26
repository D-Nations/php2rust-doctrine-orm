//! PHP Typed No Default Reflection Property - special override for typed
//! properties without a default value.
//!
//! Upstream: doctrine/persistence src/Reflection/TypedNoDefaultReflectionProperty.php

use std::ops::Deref;

use php_value::{PhpObject, Thrown, Value};

use crate::php_reflection::{PropertyAccessor, ReflectionProperty};
use crate::reflection::RuntimeReflectionProperty;

/// `Doctrine\Persistence\Reflection\TypedNoDefaultReflectionProperty`,
/// which extends [`RuntimeReflectionProperty`] (its parent through
/// `Deref`).
#[derive(Debug, Clone)]
pub struct TypedNoDefaultReflectionProperty {
    parent: RuntimeReflectionProperty,
}

impl TypedNoDefaultReflectionProperty {
    pub const CLASS: &str = "Doctrine\\Persistence\\Reflection\\TypedNoDefaultReflectionProperty";

    pub fn new(class: &str, name: &str) -> Result<Self, Thrown> {
        RuntimeReflectionProperty::new(class, name).map(Self::from)
    }

    /// `getValue($object)`.
    ///
    /// Checks that a typed property is initialized before accessing its
    /// value. This is necessary to avoid PHP error "Error: Typed property
    /// must not be accessed before initialization". Should be used only for
    /// properties without a default value.
    pub fn get_value(&self, object: &dyn PhpObject) -> Value {
        if self.parent.reflection_property().is_initialized(object) {
            self.parent.get_value(object)
        } else {
            Value::Null
        }
    }

    /// `setValue($object, $value)`.
    ///
    /// Works around the problem with setting typed no default properties
    /// to NULL which is not supported, instead `unset()` to uninitialize.
    pub fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        let property = self.parent.reflection_property();
        if value.is_null() && property.get_type().is_some_and(|t| !t.allows_null()) {
            return property.unset_value(object);
        }

        self.parent.set_value(object, value)
    }
}

impl From<RuntimeReflectionProperty> for TypedNoDefaultReflectionProperty {
    fn from(parent: RuntimeReflectionProperty) -> Self {
        Self { parent }
    }
}

impl Deref for TypedNoDefaultReflectionProperty {
    type Target = RuntimeReflectionProperty;

    fn deref(&self) -> &RuntimeReflectionProperty {
        &self.parent
    }
}

impl PropertyAccessor for TypedNoDefaultReflectionProperty {
    fn reflection(&self) -> &ReflectionProperty {
        self.parent.reflection_property()
    }

    fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        Ok(Self::get_value(self, object))
    }

    fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        Self::set_value(self, object, value)
    }

    fn as_runtime_reflection_property(&self) -> Option<&RuntimeReflectionProperty> {
        Some(&self.parent)
    }
}
