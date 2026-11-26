//! Port of `ReflectionEmbeddedPropertyTest`, on the recorded test models
//! (tests/common/models.rs); the instantiator is `EntityObject`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ReflectionEmbeddedPropertyTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::Value;
use php2rust_doctrine_orm::mapping::ReflectionEmbeddedProperty;
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_persistence::php_reflection::ReflectionProperty;

const BOOLEAN_MODEL: &str = "Doctrine\\Tests\\Models\\Generic\\BooleanModel";
const ABSTRACT_EMBEDDABLE: &str = "Doctrine\\Tests\\Models\\Reflection\\AbstractEmbeddable";
const CONCRETE_EMBEDDABLE: &str = "Doctrine\\Tests\\Models\\Reflection\\ConcreteEmbeddable";
const ARRAY_OBJECT_EXTENDING_CLASS: &str =
    "Doctrine\\Tests\\Models\\Reflection\\ArrayObjectExtendingClass";

/// `getTestedReflectionProperties()`.
fn get_tested_reflection_properties() -> Vec<(ReflectionProperty, ReflectionProperty, &'static str)>
{
    models::register();
    let property = |class: &str, name: &str| ReflectionProperty::new(class, name).unwrap();
    vec![
        (
            property(BOOLEAN_MODEL, "id"),
            property(BOOLEAN_MODEL, "id"),
            BOOLEAN_MODEL,
        ),
        // reflection on embeddables that have properties defined in abstract ancestors:
        (
            property(BOOLEAN_MODEL, "id"),
            property(ABSTRACT_EMBEDDABLE, "propertyInAbstractClass"),
            CONCRETE_EMBEDDABLE,
        ),
        (
            property(BOOLEAN_MODEL, "id"),
            property(CONCRETE_EMBEDDABLE, "propertyInConcreteClass"),
            CONCRETE_EMBEDDABLE,
        ),
        // reflection on classes extending internal PHP classes:
        (
            property(ARRAY_OBJECT_EXTENDING_CLASS, "publicProperty"),
            property(ARRAY_OBJECT_EXTENDING_CLASS, "privateProperty"),
            ARRAY_OBJECT_EXTENDING_CLASS,
        ),
        (
            property(ARRAY_OBJECT_EXTENDING_CLASS, "publicProperty"),
            property(ARRAY_OBJECT_EXTENDING_CLASS, "protectedProperty"),
            ARRAY_OBJECT_EXTENDING_CLASS,
        ),
        (
            property(ARRAY_OBJECT_EXTENDING_CLASS, "publicProperty"),
            property(ARRAY_OBJECT_EXTENDING_CLASS, "publicProperty"),
            ARRAY_OBJECT_EXTENDING_CLASS,
        ),
    ]
}

#[test]
fn testCanSetAndGetEmbeddedProperty() {
    for (index, (parent_property, child_property, embeddable_class)) in
        get_tested_reflection_properties().into_iter().enumerate()
    {
        let declaring_class = parent_property.get_declaring_class().get_name().to_owned();
        let embedded_property_reflection =
            ReflectionEmbeddedProperty::new(parent_property, child_property, embeddable_class);

        let object = EntityObject::object(&declaring_class).unwrap();

        embedded_property_reflection
            .set_value(object.as_ref(), Value::from("newValue"))
            .unwrap();

        assert_eq!(
            Value::from("newValue"),
            embedded_property_reflection
                .get_value(object.as_ref())
                .unwrap(),
            "data set #{index}"
        );

        embedded_property_reflection
            .set_value(object.as_ref(), Value::from("changedValue"))
            .unwrap();

        assert_eq!(
            Value::from("changedValue"),
            embedded_property_reflection
                .get_value(object.as_ref())
                .unwrap(),
            "data set #{index}"
        );
    }
}

#[test]
fn testWillSkipReadingPropertiesFromNullEmbeddable() {
    for (index, (parent_property, child_property, embeddable_class)) in
        get_tested_reflection_properties().into_iter().enumerate()
    {
        let declaring_class = parent_property.get_declaring_class().get_name().to_owned();
        let embedded_property_reflection =
            ReflectionEmbeddedProperty::new(parent_property, child_property, embeddable_class);

        let object = EntityObject::object(&declaring_class).unwrap();
        assert_eq!(
            Value::Null,
            embedded_property_reflection
                .get_value(object.as_ref())
                .unwrap(),
            "data set #{index}"
        );
    }
}
