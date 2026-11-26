//! Port of `ReadOnlyAccessorTest`, on the recorded test classes
//! (tests/common/models.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/PropertyAccessors/ReadOnlyAccessorTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::mapping::property_accessors::{
    PropertyAccessor, create_property_accessor,
};
use php2rust_doctrine_orm::object::EntityObject;

const READ_ONLY_CLASS: &str = "Doctrine\\Tests\\ORM\\Mapping\\PropertyAccessors\\ReadOnlyClass";
const READONLY_ACCESSOR: &str = "Doctrine\\ORM\\Mapping\\PropertyAccessors\\ReadonlyAccessor";

fn set_up() -> (Arc<EntityObject>, Arc<dyn PropertyAccessor>) {
    models::register();
    let object = EntityObject::instantiate(READ_ONLY_CLASS).unwrap();
    let accessor = create_property_accessor(READ_ONLY_CLASS, "property").unwrap();
    (object, accessor)
}

#[test]
fn testReadOnlyProperty() {
    let (object, accessor) = set_up();
    assert_eq!(READONLY_ACCESSOR, accessor.class_name());
    let handle: Object = object.clone();
    accessor.set_value(&handle, Value::Int(1)).unwrap();
    assert_eq!(Some(Value::Int(1)), object.get("property"));
    assert_eq!(Value::Int(1), accessor.get_value(&handle).unwrap());
}

#[test]
fn testReadOnlyPropertyOnlyOnce() {
    let (object, accessor) = set_up();
    assert_eq!(READONLY_ACCESSOR, accessor.class_name());
    let handle: Object = object;
    accessor.set_value(&handle, Value::Int(1)).unwrap();
    let error = accessor.set_value(&handle, Value::Int(2)).unwrap_err();
    assert_eq!("LogicException", error.class);
}
