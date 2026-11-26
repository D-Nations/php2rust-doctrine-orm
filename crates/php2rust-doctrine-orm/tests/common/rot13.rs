//! `Doctrine\Tests\DbalTypes\Rot13Type`, the DBAL type of the
//! `ValueConversionType` models.
//!
//! Upstream: doctrine/orm tests/Tests/DbalTypes/Rot13Type.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]

use std::sync::Arc;

use php_value::{PhpArray, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{
    ConversionError, Type, TypeArgument, add_type, has_type, override_type,
};

/// `Doctrine\Tests\DbalTypes\Rot13Type`: shifts every letter by 13 places
/// in the alphabet (ROT13 encoding).
#[derive(Debug)]
pub struct Rot13Type;

pub fn str_rot13(value: &Value) -> Value {
    let Value::String(s) = value else {
        return value.clone();
    };
    let shifted: Vec<u8> = s
        .as_bytes()
        .iter()
        .map(|&b| match b {
            b'a'..=b'z' => (b - b'a' + 13) % 26 + b'a',
            b'A'..=b'Z' => (b - b'A' + 13) % 26 + b'A',
            other => other,
        })
        .collect();
    Value::String(PhpString::from(shifted))
}

impl Type for Rot13Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\Rot13Type"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(str_rot13(value))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(str_rot13(value))
    }
}

/// `Type::addType('rot13', Rot13Type::class)` (or `overrideType()`).
pub fn register_rot13_type() {
    let type_ = || TypeArgument::Instance(Arc::new(Rot13Type));
    if has_type("rot13") {
        override_type("rot13", type_()).unwrap();
    } else if add_type("rot13", type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type("rot13", type_()).unwrap();
    }
}
