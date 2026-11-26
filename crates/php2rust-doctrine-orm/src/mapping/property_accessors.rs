//! The property accessors: how the ORM reads and writes an entity's
//! mapped properties (raw values, enums, embeddables, readonly and typed
//! properties without a default).
//!
//! `RawValuePropertyAccessor` and `ObjectCastPropertyAccessor` both read
//! the property's raw value (PHP's `(array) $object`) and write it through
//! reflection; property hooks do not exist in this port, so they behave
//! alike. `PropertyAccessorFactory` picks the raw value accessor (PHP
//! 8.4).
//!
//! Upstream: doctrine/orm src/Mapping/PropertyAccessors/PropertyAccessor.php src/Mapping/PropertyAccessors/PropertyAccessorFactory.php src/Mapping/PropertyAccessors/RawValuePropertyAccessor.php src/Mapping/PropertyAccessors/ObjectCastPropertyAccessor.php src/Mapping/PropertyAccessors/TypedNoDefaultPropertyAccessor.php src/Mapping/PropertyAccessors/ReadonlyAccessor.php src/Mapping/PropertyAccessors/EnumPropertyAccessor.php src/Mapping/PropertyAccessors/EmbeddablePropertyAccessor.php

use std::fmt;
use std::sync::Arc;

use php_value::object::backed_enum_from;
use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_persistence::php_reflection::{self, ReflectionProperty};

use crate::object::EntityObject;

/// `Doctrine\ORM\Mapping\PropertyAccessors\PropertyAccessor`.
pub trait PropertyAccessor: Send + Sync + fmt::Debug {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown>;

    fn get_value(&self, object: &Object) -> Result<Value, Thrown>;

    fn get_underlying_reflector(&self) -> &ReflectionProperty;

    /// The accessor's class name.
    fn class_name(&self) -> &'static str;
}

/// `$object instanceof InternalProxy && ! $object->__isInitialized()`:
/// the write goes between `__setInitialized(true)` and
/// `__setInitialized(false)`.
fn set_raw(reflection: &ReflectionProperty, object: &Object, value: Value) -> Result<(), Thrown> {
    if let Some(proxy) = php_reflection::as_proxy(object.as_ref())
        && !proxy.is_initialized()
    {
        proxy.set_initialized(true);
        let result = reflection.set_value(object.as_ref(), value);
        proxy.set_initialized(false);
        return result;
    }
    reflection.set_value(object.as_ref(), value)
}

/// `((array) $object)[$key] ?? null`.
fn get_raw(reflection: &ReflectionProperty, object: &Object) -> Value {
    php_reflection::read_property(object.as_ref(), reflection.get_name())
        .and_then(Result::ok)
        .unwrap_or_default()
}

/// `Doctrine\ORM\Mapping\PropertyAccessors\RawValuePropertyAccessor`.
#[derive(Debug, Clone)]
pub struct RawValuePropertyAccessor {
    reflection_property: ReflectionProperty,
}

impl RawValuePropertyAccessor {
    pub fn from_reflection_property(reflection_property: ReflectionProperty) -> Self {
        Self {
            reflection_property,
        }
    }
}

impl PropertyAccessor for RawValuePropertyAccessor {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown> {
        set_raw(&self.reflection_property, object, value)
    }

    fn get_value(&self, object: &Object) -> Result<Value, Thrown> {
        Ok(get_raw(&self.reflection_property, object))
    }

    fn get_underlying_reflector(&self) -> &ReflectionProperty {
        &self.reflection_property
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\RawValuePropertyAccessor"
    }
}

/// `Doctrine\ORM\Mapping\PropertyAccessors\ObjectCastPropertyAccessor`.
#[derive(Debug, Clone)]
pub struct ObjectCastPropertyAccessor {
    reflection_property: ReflectionProperty,
}

impl ObjectCastPropertyAccessor {
    /// `ObjectCastPropertyAccessor::fromNames()`.
    pub fn from_names(class: &str, name: &str) -> Result<Self, Thrown> {
        Ok(Self {
            reflection_property: ReflectionProperty::new(class, name)?,
        })
    }

    pub fn from_reflection_property(reflection_property: ReflectionProperty) -> Self {
        Self {
            reflection_property,
        }
    }
}

impl PropertyAccessor for ObjectCastPropertyAccessor {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown> {
        set_raw(&self.reflection_property, object, value)
    }

    fn get_value(&self, object: &Object) -> Result<Value, Thrown> {
        Ok(get_raw(&self.reflection_property, object))
    }

    fn get_underlying_reflector(&self) -> &ReflectionProperty {
        &self.reflection_property
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\ObjectCastPropertyAccessor"
    }
}

/// `Doctrine\ORM\Mapping\PropertyAccessors\TypedNoDefaultPropertyAccessor`:
/// null unsets the property; an uninitialized property reads as null.
#[derive(Debug, Clone)]
pub struct TypedNoDefaultPropertyAccessor {
    parent: Arc<dyn PropertyAccessor>,
    reflection_property: ReflectionProperty,
}

impl TypedNoDefaultPropertyAccessor {
    pub fn new(
        parent: Arc<dyn PropertyAccessor>,
        reflection_property: ReflectionProperty,
    ) -> Result<Self, Thrown> {
        let class = reflection_property
            .get_declaring_class()
            .get_name()
            .to_owned();
        let name = reflection_property.get_name().to_owned();
        let Some(ty) = reflection_property.get_type() else {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!(
                    "{class}::${name} must have a type when used with TypedNoDefaultPropertyAccessor"
                ),
                0,
            ));
        };
        if ty.allows_null() {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!(
                    "{class}::${name} must not be nullable when used with TypedNoDefaultPropertyAccessor"
                ),
                0,
            ));
        }
        Ok(Self {
            parent,
            reflection_property,
        })
    }
}

impl PropertyAccessor for TypedNoDefaultPropertyAccessor {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown> {
        if matches!(value, Value::Null) {
            return php_reflection::write_property(
                object.as_ref(),
                self.reflection_property.get_name(),
                None,
            );
        }
        self.parent.set_value(object, value)
    }

    fn get_value(&self, object: &Object) -> Result<Value, Thrown> {
        if self.reflection_property.is_initialized(object.as_ref()) {
            self.parent.get_value(object)
        } else {
            Ok(Value::Null)
        }
    }

    fn get_underlying_reflector(&self) -> &ReflectionProperty {
        &self.reflection_property
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\TypedNoDefaultPropertyAccessor"
    }
}

/// `Doctrine\ORM\Mapping\PropertyAccessors\ReadonlyAccessor`: a readonly
/// property is written once; writing another value is a `LogicException`.
#[derive(Debug, Clone)]
pub struct ReadonlyAccessor {
    parent: Arc<dyn PropertyAccessor>,
    reflection_property: ReflectionProperty,
}

impl ReadonlyAccessor {
    pub fn new(
        parent: Arc<dyn PropertyAccessor>,
        reflection_property: ReflectionProperty,
    ) -> Result<Self, Thrown> {
        if !reflection_property.is_readonly() {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!(
                    "{}::${} must be readonly property",
                    reflection_property.get_declaring_class().get_name(),
                    reflection_property.get_name()
                ),
                0,
            ));
        }
        Ok(Self {
            parent,
            reflection_property,
        })
    }
}

impl PropertyAccessor for ReadonlyAccessor {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown> {
        if !self.reflection_property.is_initialized(object.as_ref()) {
            return self.parent.set_value(object, value);
        }
        if !self.parent.get_value(object)?.identical(&value) {
            return Err(Thrown::new(
                "LogicException",
                format!(
                    "Attempting to change readonly property {}::${}.",
                    self.reflection_property.get_declaring_class().get_name(),
                    self.reflection_property.get_name()
                ),
                0,
            ));
        }
        Ok(())
    }

    fn get_value(&self, object: &Object) -> Result<Value, Thrown> {
        self.parent.get_value(object)
    }

    fn get_underlying_reflector(&self) -> &ReflectionProperty {
        &self.reflection_property
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\ReadonlyAccessor"
    }
}

/// `Doctrine\ORM\Mapping\PropertyAccessors\EnumPropertyAccessor`: the
/// property holds enum cases, the accessor gives and takes their backing
/// values.
#[derive(Debug, Clone)]
pub struct EnumPropertyAccessor {
    parent: Arc<dyn PropertyAccessor>,
    enum_type: String,
}

fn enum_value(case: &Value) -> Value {
    match case {
        Value::Object(object) => object.property(b"value").unwrap_or_default(),
        other => other.clone(),
    }
}

impl EnumPropertyAccessor {
    pub fn new(parent: Arc<dyn PropertyAccessor>, enum_type: impl Into<String>) -> Self {
        Self {
            parent,
            enum_type: enum_type.into(),
        }
    }

    fn from_enum(enum_value_or_list: &Value) -> Value {
        match enum_value_or_list {
            Value::Array(array) => {
                let mut values = PhpArray::new();
                for (key, case) in array.iter() {
                    values.insert(key.clone(), enum_value(case));
                }
                Value::Array(values)
            }
            case => enum_value(case),
        }
    }

    /// `$this->enumType::from($value)`: `strict` under `strict_types`
    /// (a single value), coercive from `array_map()`.
    fn from(&self, value: &Value, strict: bool) -> Result<Value, Thrown> {
        let class = php_reflection::class_info(&self.enum_type)
            .filter(|c| c.kind() == php_reflection::ClassKind::Enum)
            .ok_or_else(|| {
                Thrown::new(
                    "Error",
                    format!("Class \"{}\" not found", self.enum_type),
                    0,
                )
            })?;
        let cases = class.cases();
        if strict {
            let int_backed = cases
                .first()
                .is_some_and(|case| matches!(case.property(b"value"), Some(Value::Int(_))));
            let (accepted, wanted) = if int_backed {
                (matches!(value, Value::Int(_)), "int")
            } else {
                (matches!(value, Value::String(_)), "string")
            };
            if !accepted {
                return Err(Thrown::new(
                    "TypeError",
                    format!(
                        "{}::from(): Argument #1 ($value) must be of type {wanted}, {} given",
                        class.name(),
                        crate::mapping::property_value::value_name(value)
                    ),
                    0,
                ));
            }
        }
        match backed_enum_from(class.name(), cases, value) {
            Some(Ok(case)) => Ok(Value::Object(case)),
            Some(Err(error)) => Err(Thrown::new(error.class(), error.message(), 0)),
            None => Err(Thrown::new(
                "Error",
                format!("Call to undefined method {}::from()", class.name()),
                0,
            )),
        }
    }

    fn to_enum(&self, value: Value) -> Result<Value, Thrown> {
        match value {
            Value::Object(_) => Ok(value),
            Value::Array(array) => {
                if matches!(array.values().next(), Some(Value::Object(_))) {
                    return Ok(Value::Array(array));
                }
                let mut cases = PhpArray::new();
                for (key, item) in array.iter() {
                    cases.insert(key.clone(), self.from(item, false)?);
                }
                Ok(Value::Array(cases))
            }
            scalar => self.from(&scalar, true),
        }
    }
}

impl PropertyAccessor for EnumPropertyAccessor {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown> {
        let value = if matches!(value, Value::Null) {
            value
        } else {
            self.to_enum(value)?
        };
        self.parent.set_value(object, value)
    }

    fn get_value(&self, object: &Object) -> Result<Value, Thrown> {
        let value = self.parent.get_value(object)?;
        if matches!(value, Value::Null) {
            return Ok(Value::Null);
        }
        Ok(Self::from_enum(&value))
    }

    fn get_underlying_reflector(&self) -> &ReflectionProperty {
        self.parent.get_underlying_reflector()
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\EnumPropertyAccessor"
    }
}

/// `Doctrine\ORM\Mapping\PropertyAccessors\EmbeddablePropertyAccessor`: a
/// property of an embeddable, through the property holding it (which is
/// instantiated on the first write).
#[derive(Debug, Clone)]
pub struct EmbeddablePropertyAccessor {
    parent: Arc<dyn PropertyAccessor>,
    child: Arc<dyn PropertyAccessor>,
    embedded_class: String,
}

impl EmbeddablePropertyAccessor {
    pub fn new(
        parent: Arc<dyn PropertyAccessor>,
        child: Arc<dyn PropertyAccessor>,
        embedded_class: impl Into<String>,
    ) -> Self {
        Self {
            parent,
            child,
            embedded_class: embedded_class.into(),
        }
    }
}

impl PropertyAccessor for EmbeddablePropertyAccessor {
    fn set_value(&self, object: &Object, value: Value) -> Result<(), Thrown> {
        let embedded_object = if let Value::Object(embedded) = self.parent.get_value(object)? {
            embedded
        } else {
            let embedded = EntityObject::object(&self.embedded_class)
                .ok_or_else(|| php_reflection::class_does_not_exist(&self.embedded_class))?;
            self.parent
                .set_value(object, Value::Object(embedded.clone()))?;
            embedded
        };
        self.child.set_value(&embedded_object, value)
    }

    fn get_value(&self, object: &Object) -> Result<Value, Thrown> {
        match self.parent.get_value(object)? {
            Value::Object(embedded) => self.child.get_value(&embedded),
            _ => Ok(Value::Null),
        }
    }

    fn get_underlying_reflector(&self) -> &ReflectionProperty {
        self.child.get_underlying_reflector()
    }

    fn class_name(&self) -> &'static str {
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\EmbeddablePropertyAccessor"
    }
}

/// `PropertyAccessorFactory::createPropertyAccessor()`.
pub fn create_property_accessor(
    class_name: &str,
    property_name: &str,
) -> Result<Arc<dyn PropertyAccessor>, Thrown> {
    let reflection_property = ReflectionProperty::new(class_name, property_name)?;
    let mut accessor: Arc<dyn PropertyAccessor> = Arc::new(
        RawValuePropertyAccessor::from_reflection_property(reflection_property.clone()),
    );
    if reflection_property
        .get_type()
        .is_some_and(|ty| !ty.allows_null())
    {
        accessor = Arc::new(TypedNoDefaultPropertyAccessor::new(
            accessor,
            reflection_property.clone(),
        )?);
    }
    if reflection_property.is_readonly() {
        accessor = Arc::new(ReadonlyAccessor::new(accessor, reflection_property)?);
    }
    Ok(accessor)
}
