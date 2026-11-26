//! Port of `TypedNoDefaultPropertyAccessorTest`, on the recorded test
//! classes (tests/common/models.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/PropertyAccessors/TypedNoDefaultPropertyAccessorTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::{Object, Value};
use php2rust_doctrine_orm::mapping::property_accessors::create_property_accessor;
use php2rust_doctrine_orm::object::EntityObject;

const TYPED_CLASS: &str = "Doctrine\\Tests\\ORM\\Mapping\\PropertyAccessors\\TypedClass";

#[test]
fn testSetValueWithoutDefault() {
    models::register();
    let accessor = create_property_accessor(TYPED_CLASS, "property").unwrap();
    assert_eq!(
        "Doctrine\\ORM\\Mapping\\PropertyAccessors\\TypedNoDefaultPropertyAccessor",
        accessor.class_name()
    );
    let object: Object = EntityObject::instantiate(TYPED_CLASS).unwrap();
    accessor.set_value(&object, Value::Int(42)).unwrap();
    assert_eq!(Value::Int(42), accessor.get_value(&object).unwrap());
}

#[test]
fn testSetNullWithoutDefault() {
    models::register();
    let accessor = create_property_accessor(TYPED_CLASS, "property").unwrap();
    let object: Object = EntityObject::instantiate(TYPED_CLASS).unwrap();
    accessor.set_value(&object, Value::Null).unwrap();
    assert_eq!(Value::Null, accessor.get_value(&object).unwrap());
    accessor.set_value(&object, Value::Int(42)).unwrap();
    assert_eq!(Value::Int(42), accessor.get_value(&object).unwrap());
    accessor.set_value(&object, Value::Null).unwrap();
    assert_eq!(Value::Null, accessor.get_value(&object).unwrap());
}
