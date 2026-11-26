//! Port of `TypedEnumFieldMapperTest`, on the recorded test models
//! (tests/common/models.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/TypedEnumFieldMapperTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::php_array;
use php2rust_doctrine_orm::mapping::mapping_exception;
use php2rust_doctrine_orm::mapping::typed_field_mapper::{
    DefaultTypedFieldMapper, TypedFieldMapper,
};
use php2rust_doctrine_persistence::php_reflection::{ReflectionProperty, thrown_is_a};

#[test]
fn testNotBackedEnumThrows() {
    models::register();
    let property =
        ReflectionProperty::new("Doctrine\\Tests\\Models\\Enums\\FaultySwitch", "status").unwrap();

    let error = DefaultTypedFieldMapper::default()
        .validate_and_complete(php_array! { "fieldName" => "status" }, &property)
        .unwrap_err();

    assert!(thrown_is_a(&error, mapping_exception::CLASS), "{error:?}");
    assert!(String::from_utf8_lossy(error.message.as_bytes()).contains(
        "Attempting to map a non-backed enum type Doctrine\\Tests\\Models\\Enums\\SwitchStatus in entity Doctrine\\Tests\\Models\\Enums\\FaultySwitch::$status. Please use backed enums only"
    ));
}
