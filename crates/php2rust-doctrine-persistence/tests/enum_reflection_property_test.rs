//! Port of tests/Reflection/Reflection/EnumReflectionPropertyTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6. The
//! `ReflectionProperty` methods the class inherits are the original
//! property's (`reflection()`).
//!
//! Upstream: doctrine/persistence tests/Reflection/Reflection/EnumReflectionPropertyTest.php

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

use std::sync::Arc;

use common::{FixtureObject, FixtureProperty, assert_thrown};
use php_value::object::EnumCase;
use php_value::{PhpArray, Value};
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, IS_PUBLIC, PropertyAccessor, PropertyInfo, ReflectionProperty, register_class,
};
use php2rust_doctrine_persistence::reflection::EnumReflectionProperty;

const MY_ATTRIBUTE: &str = "Doctrine\\Tests\\Persistence\\Reflection\\MyAttribute";
const TYPED_ENUM_CLASS: &str = "Doctrine\\Tests\\Persistence\\Reflection\\TypedEnumClass";
const SUIT: &str = "Doctrine\\Tests\\Persistence\\Reflection\\Suit";

fn suit(name: &str) -> Value {
    let value = match name {
        "Hearts" => "H",
        "Diamonds" => "D",
        "Clubs" => "C",
        _ => "S",
    };
    Value::Object(EnumCase::new(SUIT, name, Some(Value::from(value))).object())
}

fn register() {
    common::register_fixtures();
    register_class(ClassInfo::class(MY_ATTRIBUTE));
    register_class(
        ClassInfo::class(TYPED_ENUM_CLASS)
            .property(
                PropertyInfo::typed("suit", SUIT, true)
                    .default_value(Value::Null)
                    .doc_comment("/** @MyDoc */")
                    .attribute(MY_ATTRIBUTE),
            )
            .property(
                PropertyInfo::typed("suits", "array", true)
                    .default_value(Value::Null)
                    .doc_comment("/** @var Suit[]|null */"),
            ),
    );
    let cases = ["Hearts", "Diamonds", "Clubs", "Spades"]
        .into_iter()
        .filter_map(|name| suit(name).as_object().cloned())
        .collect();
    register_class(ClassInfo::enumeration(SUIT, cases));
}

/// `new TypedEnumClass()`.
fn new_typed_enum_class() -> FixtureObject {
    FixtureObject::new(
        TYPED_ENUM_CLASS,
        vec![
            FixtureProperty::typed("suit", Some(Value::Null)),
            FixtureProperty::typed("suits", Some(Value::Null)),
        ],
    )
}

/// `new EnumReflectionProperty(new ReflectionProperty(TypedEnumClass::class, $name), Suit::class)`.
fn property(name: &str) -> EnumReflectionProperty {
    register();
    let original: Arc<dyn PropertyAccessor> =
        Arc::new(ReflectionProperty::new(TYPED_ENUM_CLASS, name).unwrap());
    EnumReflectionProperty::new(original, SUIT)
}

#[track_caller]
fn assert_same(expected: &Value, actual: &Value) {
    assert!(expected.identical(actual), "{expected:?} !== {actual:?}");
}

fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

#[test]
fn testGetValue() {
    let object = new_typed_enum_class();
    let refl_property = property("suit");

    assert!(refl_property.get_value(&object).unwrap().is_null());

    object.set("suit", suit("Clubs"));
    assert_same(
        &Value::from("C"),
        &refl_property.get_value(&object).unwrap(),
    );

    object.set("suit", Value::Null);
    assert!(refl_property.get_value(&object).unwrap().is_null());
}

#[test]
fn testGetDeclaringClass() {
    let refl_property = property("suit");
    assert_eq!(
        TYPED_ENUM_CLASS,
        refl_property.reflection().get_declaring_class().get_name()
    );
}

#[test]
fn testGetName() {
    let refl_property = property("suit");
    assert_eq!("suit", refl_property.reflection().get_name());
}

#[test]
fn testGetType() {
    let refl_property = property("suit");
    let ty = refl_property.reflection().get_type().unwrap();
    assert_eq!(SUIT, ty.get_name());
}

#[test]
fn testGetAttributes() {
    let refl_property = property("suit");
    assert_eq!(1, refl_property.reflection().get_attributes().len());
    assert_eq!(MY_ATTRIBUTE, refl_property.reflection().get_attributes()[0]);
}

#[test]
fn testSetValidValue() {
    let object = new_typed_enum_class();
    object.set("suit", suit("Hearts"));

    let refl_property = property("suit");

    refl_property.set_value(&object, Value::Null).unwrap();
    assert!(refl_property.get_value(&object).unwrap().is_null());
    assert!(object.get("suit").unwrap().is_null());

    refl_property.set_value(&object, Value::from("D")).unwrap();
    assert_same(
        &Value::from("D"),
        &refl_property.get_value(&object).unwrap(),
    );
    assert_same(&suit("Diamonds"), &object.get("suit").unwrap());
}

#[test]
fn testSetInvalidValue() {
    let object = new_typed_enum_class();
    let refl_property = property("suit");

    assert_thrown(
        refl_property.set_value(&object, Value::from("A")),
        "ValueError",
        "",
    );
}

#[test]
fn testSetValidArrayValue() {
    let object = new_typed_enum_class();
    object.set("suits", list(vec![suit("Hearts"), suit("Clubs")]));

    let refl_property = property("suits");

    refl_property.set_value(&object, Value::Null).unwrap();
    assert!(refl_property.get_value(&object).unwrap().is_null());
    assert!(object.get("suits").unwrap().is_null());

    refl_property.set_value(&object, list(vec![])).unwrap();
    assert_same(&list(vec![]), &refl_property.get_value(&object).unwrap());
    assert_same(&list(vec![]), &object.get("suits").unwrap());

    refl_property
        .set_value(&object, list(vec![Value::from("H"), Value::from("D")]))
        .unwrap();
    assert_same(
        &list(vec![Value::from("H"), Value::from("D")]),
        &refl_property.get_value(&object).unwrap(),
    );
    assert_same(
        &list(vec![suit("Hearts"), suit("Diamonds")]),
        &object.get("suits").unwrap(),
    );
}

#[test]
fn testSetEnum() {
    let object = new_typed_enum_class();
    let refl_property = property("suit");
    refl_property.set_value(&object, suit("Hearts")).unwrap();

    assert_same(&suit("Hearts"), &object.get("suit").unwrap());
}

#[test]
fn testSetEnumArray() {
    let object = new_typed_enum_class();
    let refl_property = property("suits");
    refl_property
        .set_value(&object, list(vec![suit("Hearts"), suit("Diamonds")]))
        .unwrap();

    assert_same(
        &list(vec![suit("Hearts"), suit("Diamonds")]),
        &object.get("suits").unwrap(),
    );
}

#[test]
fn testGetModifiers() {
    let refl_property = property("suit");
    assert_eq!(IS_PUBLIC, refl_property.reflection().get_modifiers());
}

#[test]
fn testGetDocComment() {
    let refl_property = property("suit");
    let comment = refl_property.reflection().get_doc_comment().unwrap();
    assert!(comment.contains("@MyDoc"));
}

#[test]
fn testIsPrivate() {
    let refl_property = property("suit");
    assert!(!refl_property.reflection().is_private());
}

#[test]
fn testIsPublic() {
    let refl_property = property("suit");
    assert!(refl_property.reflection().is_public());
}

#[test]
fn testHasDefaultValue() {
    let refl_property = property("suit");
    assert!(refl_property.reflection().has_default_value());
}
