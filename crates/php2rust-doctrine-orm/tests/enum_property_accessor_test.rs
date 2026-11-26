//! Port of `EnumPropertyAccessorTest`, on the recorded test classes
//! (tests/common/models.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/PropertyAccessors/EnumPropertyAccessorTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::object::EnumCase;
use php_value::vectors::phpunit_equals;
use php_value::{Object, Value, php_array};
use php2rust_doctrine_orm::mapping::property_accessors::{
    EnumPropertyAccessor, PropertyAccessor, create_property_accessor,
};
use php2rust_doctrine_orm::object::EntityObject;

const ENUM_CLASS: &str = "Doctrine\\Tests\\ORM\\Mapping\\PropertyAccessors\\EnumClass";
const ENUM_TYPE: &str = "Doctrine\\Tests\\ORM\\Mapping\\PropertyAccessors\\EnumType";

/// `EnumType::$case`.
fn case(name: &str) -> Value {
    Value::Object(EnumCase::new(ENUM_TYPE, name, Some(Value::from(name.to_lowercase()))).object())
}

fn assert_equals(expected: &Value, actual: &Value) {
    assert!(
        phpunit_equals(expected, actual),
        "{expected:?} != {actual:?}"
    );
}

fn set_up(property: &str) -> (std::sync::Arc<EntityObject>, EnumPropertyAccessor) {
    models::register();
    let object = EntityObject::instantiate(ENUM_CLASS).unwrap();
    let accessor = create_property_accessor(ENUM_CLASS, property).unwrap();
    (object, EnumPropertyAccessor::new(accessor, ENUM_TYPE))
}

#[test]
fn testEnumSetEnumGetValue() {
    let (object, accessor) = set_up("enum");
    let handle: Object = object.clone();
    accessor.set_value(&handle, case("A")).unwrap();
    assert_equals(&object.get("enum").unwrap(), &case("A"));
    assert_equals(&Value::from("a"), &accessor.get_value(&handle).unwrap());
}

#[test]
fn testEnumSetDatabaseGetValue() {
    let (object, accessor) = set_up("enum");
    let handle: Object = object.clone();
    accessor.set_value(&handle, Value::from("a")).unwrap();
    assert_equals(&object.get("enum").unwrap(), &case("A"));
    assert_equals(&Value::from("a"), &accessor.get_value(&handle).unwrap());
}

#[test]
fn testEnumSetDatabaseArrayGetValue() {
    let (object, accessor) = set_up("enumList");
    let handle: Object = object.clone();
    let values = Value::Array(php_array!["a", "b", "c"]);
    accessor.set_value(&handle, values.clone()).unwrap();
    assert_equals(
        &object.get("enumList").unwrap(),
        &Value::Array(php_array![case("A"), case("B"), case("C")]),
    );
    assert_equals(&values, &accessor.get_value(&handle).unwrap());
}
