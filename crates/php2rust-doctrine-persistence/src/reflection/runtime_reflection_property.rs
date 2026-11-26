//! PHP Runtime Reflection Property.
//!
//! Avoids triggering lazy loading if the provided object is a
//! [`Proxy`](crate::Proxy).
//!
//! PHP reads the value from the `(array)` cast of the object, which never
//! runs `__get()` or hooks: here it is the raw value of the registered
//! class (`php_reflection::ReflectionObject`), and `null` when the
//! property is not initialized.
//!
//! Upstream: doctrine/persistence src/Reflection/RuntimeReflectionProperty.php

use php_value::{PhpObject, Thrown, Value};

use crate::php_reflection::{self, PropertyAccessor, ReflectionProperty};

/// `Doctrine\Persistence\Reflection\RuntimeReflectionProperty`.
#[derive(Debug, Clone)]
pub struct RuntimeReflectionProperty {
    property: ReflectionProperty,
}

impl RuntimeReflectionProperty {
    pub const CLASS: &str = "Doctrine\\Persistence\\Reflection\\RuntimeReflectionProperty";

    /// `new RuntimeReflectionProperty($class, $name)`: the
    /// `ReflectionException` when there is no such property.
    pub fn new(class: &str, name: &str) -> Result<Self, Thrown> {
        Ok(Self {
            property: ReflectionProperty::new(class.trim_start_matches('\\'), name)?,
        })
    }

    /// The parent class.
    pub fn reflection_property(&self) -> &ReflectionProperty {
        &self.property
    }

    /// `getValue($object)`.
    pub fn get_value(&self, object: &dyn PhpObject) -> Value {
        match php_reflection::read_property(object, self.property.get_name()) {
            Some(Ok(value)) => value,
            _ => Value::Null,
        }
    }

    /// `setValue($object, $value)`: on a proxy that is not initialized,
    /// with `__setInitialized()` around the write (and no write when the
    /// proxy has no such method).
    pub fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        let Some(proxy) = php_reflection::as_proxy(object).filter(|p| !p.is_initialized()) else {
            return self.property.set_value(object, value);
        };

        if proxy.set_initialized(true).is_none() {
            return Ok(());
        }

        let result = self.property.set_value(object, value);
        proxy.set_initialized(false);
        result
    }
}

impl PropertyAccessor for RuntimeReflectionProperty {
    fn reflection(&self) -> &ReflectionProperty {
        &self.property
    }

    fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        Ok(Self::get_value(self, object))
    }

    fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        Self::set_value(self, object, value)
    }

    fn as_runtime_reflection_property(&self) -> Option<&RuntimeReflectionProperty> {
        Some(self)
    }
}
