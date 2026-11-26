//! The legacy reflection properties that `LegacyReflectionFields` hands
//! out: a readonly property that accepts a second write of the same
//! value, and a property of an embedded object, reached through the
//! property that holds it.
//!
//! Upstream: doctrine/orm src/Mapping/ReflectionReadonlyProperty.php src/Mapping/ReflectionEmbeddedProperty.php

use php_value::{PhpObject, Thrown, Value};
use php2rust_doctrine_persistence::php_reflection::ReflectionProperty;

use crate::object::EntityObject;

/// `Doctrine\ORM\Mapping\ReflectionReadonlyProperty`.
#[derive(Debug, Clone)]
pub struct ReflectionReadonlyProperty {
    wrapped_property: ReflectionProperty,
}

impl ReflectionReadonlyProperty {
    pub fn new(wrapped_property: ReflectionProperty) -> Result<Self, Thrown> {
        if !wrapped_property.is_readonly() {
            return Err(Thrown::new(
                "InvalidArgumentException",
                "Given property is not readonly.",
                0,
            ));
        }
        Ok(Self { wrapped_property })
    }

    /// `getValue()`.
    pub fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        self.wrapped_property.get_value(object)
    }

    /// `setValue()`: a write to an initialized property must keep its
    /// value.
    pub fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        if !self.wrapped_property.is_initialized(object) {
            return self.wrapped_property.set_value(object, value);
        }

        if !self.wrapped_property.get_value(object)?.identical(&value) {
            return Err(Thrown::new(
                "LogicException",
                format!(
                    "Attempting to change readonly property {}::${}.",
                    self.wrapped_property.class(),
                    self.wrapped_property.get_name()
                ),
                0,
            ));
        }
        Ok(())
    }
}

/// `Doctrine\ORM\Mapping\ReflectionEmbeddedProperty`.
#[derive(Debug, Clone)]
pub struct ReflectionEmbeddedProperty {
    parent_property: ReflectionProperty,
    child_property: ReflectionProperty,
    embedded_class: String,
}

impl ReflectionEmbeddedProperty {
    /// `$parentProperty` is the property of the class where the embedded
    /// object has to be put, `$childProperty` the property of the
    /// embedded object.
    pub fn new(
        parent_property: ReflectionProperty,
        child_property: ReflectionProperty,
        embedded_class: impl Into<String>,
    ) -> Self {
        Self {
            parent_property,
            child_property,
            embedded_class: embedded_class.into(),
        }
    }

    /// `getValue()`: null while the embedded object is null.
    pub fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        match self.parent_property.get_value(object)? {
            Value::Object(embedded_object) => {
                self.child_property.get_value(embedded_object.as_ref())
            }
            _ => Ok(Value::Null),
        }
    }

    /// `setValue()`: instantiates the embedded object when it is null.
    pub fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        let embedded_object = match self.parent_property.get_value(object)? {
            Value::Object(embedded_object) => embedded_object,
            _ => {
                let embedded_object =
                    EntityObject::object(&self.embedded_class).ok_or_else(|| {
                        Thrown::new(
                            "Doctrine\\Instantiator\\Exception\\InvalidArgumentException",
                            format!(
                                "The provided class \"{}\" does not exist",
                                self.embedded_class
                            ),
                            0,
                        )
                    })?;
                self.parent_property
                    .set_value(object, Value::Object(embedded_object.clone()))?;
                embedded_object
            }
        };

        self.child_property
            .set_value(embedded_object.as_ref(), value)
    }
}
