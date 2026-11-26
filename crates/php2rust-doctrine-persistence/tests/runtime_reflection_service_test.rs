//! Port of tests/Mapping/RuntimeReflectionServiceTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6. The test
//! case's class is declared with its properties and public methods, and
//! `$this` is a `FixtureObject` of it.
//!
//! Upstream: doctrine/persistence tests/Mapping/RuntimeReflectionServiceTest.php

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
use php_value::Value;
use php2rust_doctrine_persistence::mapping::{
    ReflectionService, RuntimeReflectionService, mapping_exception,
};
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, MethodInfo, PropertyAccessor, PropertyInfo, Visibility, register_class,
};
use php2rust_doctrine_persistence::reflection::TypedNoDefaultReflectionProperty;

const SELF: &str = "Doctrine\\Tests\\Persistence\\Mapping\\RuntimeReflectionServiceTest";
const TEST_CASE: &str = "PHPUnit\\Framework\\TestCase";
const ASSERT: &str = "PHPUnit\\Framework\\Assert";

const TESTS: [&str; 13] = [
    "testShortname",
    "testClassNamespaceName",
    "testGetParentClasses",
    "testGetParentClassesForAbsentClass",
    "testGetMethods",
    "testGetAccessibleProperty",
    "testGetTypedNoDefaultReflectionProperty",
    "testGetTypedDefaultReflectionProperty",
    "testGetTypedPublicNoDefaultPropertyWorksWithGetValue",
    "testGetNonTypedNoDefaultReflectionProperty",
    "testGetNonTypedDefaultReflectionProperty",
    "testGetTypedPublicDefaultPropertyWorksWithGetValue",
    "testGetNonTypedPublicDefaultPropertyWorksWithGetValue",
];

fn register() {
    common::register_fixtures();
    register_class(ClassInfo::class(ASSERT));
    register_class(ClassInfo::class(TEST_CASE).extends(ASSERT));
    let empty = || Value::from("");
    let mut class = ClassInfo::class(SELF)
        .extends(TEST_CASE)
        .property(
            PropertyInfo::typed("reflectionService", RuntimeReflectionService::CLASS, false)
                .private(),
        )
        .property(PropertyInfo::typed("unusedPublicProperty", "mixed", true))
        .property(
            PropertyInfo::typed("typedNoDefaultProperty", "string", false)
                .private()
                .readonly(),
        )
        .property(
            PropertyInfo::typed("typedDefaultProperty", "string", false)
                .private()
                .default_value(empty()),
        )
        .property(
            PropertyInfo::typed("nonTypedNoDefaultProperty", "string", false)
                .private()
                .readonly(),
        )
        .property(
            PropertyInfo::typed("nonTypedDefaultProperty", "string", false)
                .private()
                .default_value(empty()),
        )
        .property(PropertyInfo::typed(
            "typedNoDefaultPublicProperty",
            "string",
            false,
        ))
        .property(
            PropertyInfo::typed("typedDefaultPublicProperty", "string", false)
                .default_value(empty()),
        )
        .property(PropertyInfo::typed(
            "nonTypedNoDefaultPublicProperty",
            "string",
            false,
        ))
        .property(
            PropertyInfo::typed("nonTypedDefaultPublicProperty", "string", false)
                .default_value(empty()),
        )
        .method(MethodInfo::new("setUp").visibility(Visibility::Protected));
    for test in TESTS {
        class = class.method(MethodInfo::new(test));
    }
    register_class(class);
}

/// `$this`, after `setUp()`.
fn this() -> FixtureObject {
    FixtureObject::new(
        SELF,
        vec![
            FixtureProperty::typed(
                "reflectionService",
                Some(Value::Object(Arc::new(RuntimeReflectionService::new()))),
            )
            .private(),
            FixtureProperty::typed("unusedPublicProperty", None),
            FixtureProperty::typed("typedNoDefaultProperty", None).private(),
            FixtureProperty::typed("typedDefaultProperty", Some(Value::from(""))).private(),
            FixtureProperty::typed("nonTypedNoDefaultProperty", None).private(),
            FixtureProperty::typed("nonTypedDefaultProperty", Some(Value::from(""))).private(),
            FixtureProperty::typed("typedNoDefaultPublicProperty", None),
            FixtureProperty::typed("typedDefaultPublicProperty", Some(Value::from(""))),
            FixtureProperty::typed("nonTypedNoDefaultPublicProperty", None),
            FixtureProperty::typed("nonTypedDefaultPublicProperty", Some(Value::from(""))),
        ],
    )
}

fn set_up() -> RuntimeReflectionService {
    register();
    RuntimeReflectionService::new()
}

fn is_typed_no_default(property: &dyn PropertyAccessor) -> bool {
    (property as &dyn std::any::Any)
        .downcast_ref::<TypedNoDefaultReflectionProperty>()
        .is_some()
}

#[test]
fn testShortname() {
    let reflection_service = set_up();
    assert_eq!(
        "RuntimeReflectionServiceTest",
        reflection_service.get_class_short_name(SELF).unwrap()
    );
}

#[test]
fn testClassNamespaceName() {
    let reflection_service = set_up();
    assert_eq!(
        "Doctrine\\Tests\\Persistence\\Mapping",
        reflection_service.get_class_namespace(SELF).unwrap()
    );
}

#[test]
fn testGetParentClasses() {
    let reflection_service = set_up();
    let classes = reflection_service.get_parent_classes(SELF).unwrap();
    assert!(
        !classes.is_empty(),
        "The test class {SELF} should have at least one parent."
    );
}

#[test]
fn testGetParentClassesForAbsentClass() {
    let reflection_service = set_up();
    assert_thrown(
        reflection_service.get_parent_classes("Doctrine\\Tests\\Persistence\\Mapping\\AbsentClass"),
        mapping_exception::CLASS,
        "",
    );
}

#[test]
fn testGetMethods() {
    let reflection_service = set_up();
    assert!(reflection_service.has_public_method(SELF, "testGetMethods"));
    assert!(!reflection_service.has_public_method(SELF, "testGetMethods2"));
}

#[test]
fn testGetAccessibleProperty() {
    let reflection_service = set_up();
    let this = this();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "reflectionService")
        .unwrap()
        .unwrap();
    let value = refl_prop.get_value(&this).unwrap();
    assert!(
        value
            .as_object()
            .and_then(|o| o.downcast_ref::<RuntimeReflectionService>())
            .is_some()
    );

    let refl_prop = reflection_service
        .get_accessible_property(SELF, "unusedPublicProperty")
        .unwrap()
        .unwrap();
    assert!(refl_prop.as_runtime_reflection_property().is_some());
}

#[test]
fn testGetTypedNoDefaultReflectionProperty() {
    let reflection_service = set_up();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "typedNoDefaultProperty")
        .unwrap()
        .unwrap();
    assert!(is_typed_no_default(&*refl_prop));
}

#[test]
fn testGetTypedDefaultReflectionProperty() {
    let reflection_service = set_up();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "typedDefaultProperty")
        .unwrap()
        .unwrap();
    assert!(!is_typed_no_default(&*refl_prop));
}

#[test]
fn testGetTypedPublicNoDefaultPropertyWorksWithGetValue() {
    let reflection_service = set_up();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "typedNoDefaultPublicProperty")
        .unwrap()
        .unwrap();
    assert!(refl_prop.as_runtime_reflection_property().is_some());
    assert!(is_typed_no_default(&*refl_prop));
    assert!(refl_prop.get_value(&this()).unwrap().is_null());
}

#[test]
fn testGetNonTypedNoDefaultReflectionProperty() {
    let reflection_service = set_up();
    let refl_prop = reflection_service.get_accessible_property(SELF, "nonTypedNoDefaultProperty");
    assert!(refl_prop.unwrap().is_some());
}

#[test]
fn testGetNonTypedDefaultReflectionProperty() {
    let reflection_service = set_up();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "nonTypedDefaultProperty")
        .unwrap()
        .unwrap();
    assert!(!is_typed_no_default(&*refl_prop));
}

#[test]
fn testGetTypedPublicDefaultPropertyWorksWithGetValue() {
    let reflection_service = set_up();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "typedDefaultPublicProperty")
        .unwrap()
        .unwrap();
    assert!(!is_typed_no_default(&*refl_prop));
}

#[test]
fn testGetNonTypedPublicDefaultPropertyWorksWithGetValue() {
    let reflection_service = set_up();
    let refl_prop = reflection_service
        .get_accessible_property(SELF, "nonTypedDefaultPublicProperty")
        .unwrap()
        .unwrap();
    assert!(refl_prop.as_runtime_reflection_property().is_some());
}
