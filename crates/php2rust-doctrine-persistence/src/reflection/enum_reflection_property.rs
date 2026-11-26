//! PHP Enum Reflection Property - special override for backed enums.
//!
//! `EnumReflectionProperty.php` declares `strict_types=1`: `from()` on a
//! single value takes only the backing type (the `TypeError` otherwise),
//! while `array_map([$enumType, 'from'], ...)` calls it from PHP's engine,
//! with coercion (`php_value::object::backed_enum_from`; `null` becomes
//! `0`, without PHP's deprecation notice). The `TypeError`
//! of `toEnum()`'s parameter type ends with the type given; PHP adds the
//! calling file and line.
//!
//! Upstream: doctrine/persistence src/Reflection/EnumReflectionProperty.php

use std::sync::Arc;

use php_value::object::{EnumCase, backed_enum_from};
use php_value::{Object, PhpArray, PhpObject, Thrown, Value};

use crate::php_reflection::{self, PropertyAccessor, ReflectionProperty};

/// `Doctrine\Persistence\Reflection\EnumReflectionProperty`.
#[derive(Debug, Clone)]
pub struct EnumReflectionProperty {
    original_reflection_property: Arc<dyn PropertyAccessor>,
    enum_type: String,
}

impl EnumReflectionProperty {
    pub const CLASS: &str = "Doctrine\\Persistence\\Reflection\\EnumReflectionProperty";

    /// `enum_type` is the backed enum's class name.
    pub fn new(original_reflection_property: Arc<dyn PropertyAccessor>, enum_type: &str) -> Self {
        Self {
            original_reflection_property,
            enum_type: enum_type.to_owned(),
        }
    }

    /// `getDeclaringClass()` and the other `ReflectionProperty` methods
    /// are the original property's (the constructor passes its class and
    /// name to the parent).
    pub fn reflection_property(&self) -> &ReflectionProperty {
        self.original_reflection_property.reflection()
    }

    /// `getValue($object)`: the backing value, or the backing values of an
    /// array of enums.
    pub fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        let value = self.original_reflection_property.get_value(object)?;
        if value.is_null() {
            return Ok(Value::Null);
        }

        Self::from_enum(&value)
    }

    /// `setValue($object, $value)`: a backing value (or an array of them)
    /// becomes the enum case.
    pub fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        let value = if value.is_null() {
            value
        } else {
            self.to_enum(value)?
        };

        self.original_reflection_property.set_value(object, value)
    }

    fn from_enum(value: &Value) -> Result<Value, Thrown> {
        if let Value::Array(enums) = value {
            let mut values = PhpArray::new();
            for (key, enum_value) in enums.iter() {
                values.insert(key.clone(), Self::backing_value(enum_value, true)?);
            }
            return Ok(Value::Array(values));
        }

        Self::backing_value(value, false)
    }

    /// `$enum->value`; the `TypeError`s of the parameter types
    /// (`BackedEnum|array` for `fromEnum()`, `BackedEnum` for the
    /// `array_map()` closure).
    fn backing_value(value: &Value, in_array: bool) -> Result<Value, Thrown> {
        if let Some(case) = value
            .as_object()
            .and_then(|o| o.downcast_ref::<EnumCase>())
            .filter(|c| c.value().is_some())
        {
            return Ok(case.value().cloned().unwrap_or_default());
        }
        let message = if in_array {
            format!(
                "{}::{{closure:{}::fromEnum():{}}}(): Argument #1 ($enum) must be of type BackedEnum, {} given",
                Self::CLASS,
                Self::CLASS,
                FROM_ENUM_CLOSURE_LINE,
                given(value)
            )
        } else {
            format!(
                "{}::fromEnum(): Argument #1 ($enum) must be of type BackedEnum|array, {} given",
                Self::CLASS,
                given(value)
            )
        };
        Err(Thrown::new("TypeError", message, 0))
    }

    fn to_enum(&self, value: Value) -> Result<Value, Thrown> {
        if is_backed_enum(&value) {
            return Ok(value);
        }

        if let Value::Array(values) = &value {
            if values.first().is_some_and(|(_, v)| is_backed_enum(v)) {
                return Ok(value);
            }

            let mut enums = PhpArray::new();
            for (key, v) in values.iter() {
                // `null` to `int|string`: PHP coerces it to `0` (with a
                // deprecation notice).
                let v = if v.is_null() { &Value::Int(0) } else { v };
                enums.insert(key.clone(), Value::Object(self.from(v, false)?));
            }
            return Ok(Value::Array(enums));
        }

        match value {
            Value::Int(_) | Value::String(_) => Ok(Value::Object(self.from(&value, true)?)),
            other => Err(Thrown::new(
                "TypeError",
                format!(
                    "{}::toEnum(): Argument #1 ($value) must be of type BackedEnum|array|string|int, {} given",
                    Self::CLASS,
                    given(&other)
                ),
                0,
            )),
        }
    }

    /// `$this->enumType::from($value)`.
    fn from(&self, value: &Value, strict: bool) -> Result<Object, Thrown> {
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
        let int_backed = cases.iter().any(|c| {
            matches!(
                c.downcast_ref::<EnumCase>().and_then(EnumCase::value),
                Some(Value::Int(_))
            )
        });
        if strict {
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
                        given(value)
                    ),
                    0,
                ));
            }
        }
        match backed_enum_from(class.name(), cases, value) {
            Some(Ok(case)) => Ok(case),
            Some(Err(error)) => Err(Thrown::new(error.class(), error.message(), 0)),
            None => Err(Thrown::new(
                "Error",
                format!("Call to undefined method {}::from()", class.name()),
                0,
            )),
        }
    }
}

/// The line of the `array_map()` closure in `fromEnum()`.
const FROM_ENUM_CLOSURE_LINE: u32 = 73;

/// The type a `TypeError` names (`true` and `false` for booleans).
fn given(value: &Value) -> String {
    match value {
        Value::Bool(true) => "true".to_owned(),
        Value::Bool(false) => "false".to_owned(),
        other => other.debug_type(),
    }
}

/// `$value instanceof BackedEnum`.
fn is_backed_enum(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|o| o.downcast_ref::<EnumCase>())
        .is_some_and(|c| c.value().is_some())
}

impl PropertyAccessor for EnumReflectionProperty {
    fn reflection(&self) -> &ReflectionProperty {
        self.reflection_property()
    }

    fn get_value(&self, object: &dyn PhpObject) -> Result<Value, Thrown> {
        Self::get_value(self, object)
    }

    fn set_value(&self, object: &dyn PhpObject, value: Value) -> Result<(), Thrown> {
        Self::set_value(self, object, value)
    }

    fn is_initialized(&self, object: &dyn PhpObject) -> bool {
        self.original_reflection_property.is_initialized(object)
    }
}
