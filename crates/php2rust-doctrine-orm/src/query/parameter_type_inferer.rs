//! `ParameterTypeInferer`.
//!
//! Upstream: doctrine/orm src/Query/ParameterTypeInferer.php

use php_value::date_interval::DateInterval;
use php_value::datetime::{DateTime, DateTimeImmutable};
use php_value::object::EnumCase;
use php_value::{PhpObject, Value};
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_dbal::{ArrayParameterType, ParameterType, WrapperParameterType};

/// Provides an enclosed support for parameter inferring.
#[derive(Debug)]
pub struct ParameterTypeInferer {
    _private: (),
}

/// `$value instanceof $class` for a PHP class `php_value` models with a
/// Rust type, or one an object says it extends.
fn instance_of<T: PhpObject>(object: &dyn PhpObject, class: &str) -> bool {
    object.downcast_ref::<T>().is_some() || object.instance_of(class)
}

/// The backing value of a `BackedEnum` case.
fn backed_enum_value(value: &Value) -> Option<&Value> {
    value
        .as_object()
        .and_then(|o| o.downcast_ref::<EnumCase>())
        .and_then(EnumCase::value)
}

impl ParameterTypeInferer {
    /// Infers the type of a given value: a `ParameterType`, an
    /// `ArrayParameterType` or the name of a `Types` type.
    pub fn infer_type(value: &Value) -> WrapperParameterType {
        if matches!(value, Value::Int(_)) {
            return Types::INTEGER.into();
        }
        if matches!(value, Value::Bool(_)) {
            return Types::BOOLEAN.into();
        }
        if let Value::Object(object) = value {
            let object = object.as_ref();
            if instance_of::<DateTimeImmutable>(object, "DateTimeImmutable") {
                return Types::DATETIME_IMMUTABLE.into();
            }
            if instance_of::<DateTime>(object, "DateTimeInterface") {
                return Types::DATETIME_MUTABLE.into();
            }
            if instance_of::<DateInterval>(object, "DateInterval") {
                return Types::DATEINTERVAL.into();
            }
        }
        if let Some(backing) = backed_enum_value(value) {
            return if matches!(backing, Value::Int(_)) {
                Types::INTEGER.into()
            } else {
                Types::STRING.into()
            };
        }
        if let Value::Array(array) = value {
            // current(): the first element (`false` for an empty array).
            let first_value = array.values().next();
            let first_value = first_value.map(|first| backed_enum_value(first).unwrap_or(first));
            return if matches!(first_value, Some(Value::Int(_))) {
                ArrayParameterType::Integer.into()
            } else {
                ArrayParameterType::String.into()
            };
        }
        ParameterType::String.into()
    }
}
