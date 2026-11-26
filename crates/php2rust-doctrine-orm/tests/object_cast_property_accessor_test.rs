//! Port of `ObjectCastPropertyAccessorTest`, on the recorded test classes
//! (tests/common/models.rs). `ObjectClassInternalProxy`'s
//! `$isInitialized` and `$counter` are the proxy state of `EntityObject`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/PropertyAccessors/ObjectCastPropertyAccessorTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::{Object, Value};
use php2rust_doctrine_orm::mapping::property_accessors::{
    ObjectCastPropertyAccessor, PropertyAccessor,
};
use php2rust_doctrine_orm::object::EntityObject;

const NS: &str = "Doctrine\\Tests\\ORM\\Mapping\\PropertyAccessors\\";

fn set_up(
    class: &str,
    property: &str,
) -> (std::sync::Arc<EntityObject>, ObjectCastPropertyAccessor) {
    models::register();
    let class = format!("{NS}{class}");
    let object = EntityObject::instantiate(&class).unwrap();
    (
        object,
        ObjectCastPropertyAccessor::from_names(&class, property).unwrap(),
    )
}

#[test]
fn testSetGetPublicPropertyValue() {
    let (object, accessor) = set_up("ObjectClass", "property");
    let handle: Object = object.clone();
    accessor.set_value(&handle, Value::from("value")).unwrap();
    assert_eq!(Some(Value::from("value")), object.get("property"));
    assert_eq!(Value::from("value"), accessor.get_value(&handle).unwrap());
}

#[test]
fn testSetGetPrivatePropertyValue() {
    let (object, accessor) = set_up("ObjectClass", "property2");
    let handle: Object = object.clone();
    accessor.set_value(&handle, Value::from("value")).unwrap();
    // getProperty2()
    assert_eq!(Some(Value::from("value")), object.get("property2"));
    assert_eq!(Value::from("value"), accessor.get_value(&handle).unwrap());
}

#[test]
fn testSetGetInternalProxyValue() {
    let (object, accessor) = set_up("ObjectClassInternalProxy", "property");
    let handle: Object = object.clone();
    accessor.set_value(&handle, Value::from("value")).unwrap();
    assert_eq!(Some(Value::from("value")), object.get("property"));
    assert_eq!(Value::from("value"), accessor.get_value(&handle).unwrap());
    let (is_initialized, counter) = object.proxy_state().unwrap();
    assert!(!is_initialized);
    assert_eq!(2, counter);
}
