//! Port of tests/Reflection/TypedNoDefaultReflectionPropertyTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6.
//!
//! Upstream: doctrine/persistence tests/Reflection/TypedNoDefaultReflectionPropertyTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(
    clippy::doc_markdown,
    clippy::disallowed_names,
    reason = "upstream's test, class and field names"
)]
#![allow(non_snake_case)]

mod common;

use common::{FixtureObject, FixtureProperty};
use php_value::Value;
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, MethodInfo, PropertyInfo, register_class,
};
use php2rust_doctrine_persistence::reflection::TypedNoDefaultReflectionProperty;

const TEST_CLASS: &str =
    "Doctrine\\Tests\\Persistence\\Reflection\\TypedNoDefaultReflectionPropertyTestClass";
const TYPED_FOO: &str = "Doctrine\\Tests\\Persistence\\Reflection\\TypedFoo";
const TYPED_NULLABLE_FOO: &str = "Doctrine\\Tests\\Persistence\\Reflection\\TypedNullableFoo";

fn register() {
    common::register_fixtures();
    register_class(
        ClassInfo::class(TEST_CLASS).property(PropertyInfo::typed("test", "string", false)),
    );
    register_class(
        ClassInfo::class(TYPED_FOO)
            .property(PropertyInfo::typed("id", "int", false).private())
            .method(MethodInfo::new("setId")),
    );
    register_class(
        ClassInfo::class(TYPED_NULLABLE_FOO)
            .property(
                PropertyInfo::typed("value", "string", true)
                    .private()
                    .default_value(Value::Null),
            )
            .method(MethodInfo::new("setValue"))
            .method(MethodInfo::new("getValue")),
    );
}

#[test]
fn testGetValue() {
    register();
    let object = FixtureObject::new(TEST_CLASS, vec![FixtureProperty::typed("test", None)]);

    let refl_property = TypedNoDefaultReflectionProperty::new(TEST_CLASS, "test").unwrap();

    assert!(refl_property.get_value(&object).is_null());

    object.set("test", Value::from("testValue"));

    assert!(
        refl_property
            .get_value(&object)
            .identical(&Value::from("testValue"))
    );

    object.unset("test");

    assert!(refl_property.get_value(&object).is_null());
}

#[test]
fn testSetValueNull() {
    register();
    let reflection = TypedNoDefaultReflectionProperty::new(TYPED_FOO, "id").unwrap();

    let object = FixtureObject::new(
        TYPED_FOO,
        vec![FixtureProperty::typed("id", None).private()],
    );
    // $object->setId(1)
    object.set("id", Value::Int(1));

    assert!(reflection.reflection_property().is_initialized(&object));

    reflection.set_value(&object, Value::Null).unwrap();

    assert!(reflection.get_value(&object).is_null());
    assert!(!reflection.reflection_property().is_initialized(&object));
}

#[test]
fn testSetValueNullOnNullableProperty() {
    register();
    let reflection = TypedNoDefaultReflectionProperty::new(TYPED_NULLABLE_FOO, "value").unwrap();

    let object = FixtureObject::new(
        TYPED_NULLABLE_FOO,
        vec![FixtureProperty::typed("value", Some(Value::Null)).private()],
    );

    reflection.set_value(&object, Value::Null).unwrap();

    assert!(reflection.get_value(&object).is_null());
    assert!(reflection.reflection_property().is_initialized(&object));
    // $object->getValue()
    assert!(object.get("value").unwrap().is_null());
}
