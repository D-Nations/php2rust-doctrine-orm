//! InstantiatorTest.php at doctrine/instantiator
//! 23da848e1a2308728fe5fdddabf4be17ff9720c7. Names, providers and
//! assertions are PHPUnit's; a provider's rows run in one test, labelled.
//! The assets, and the PHP classes they extend, are declared in the class
//! registry (`support::declare_assets()`); `PDORow` is declared as PHP
//! declares it (internal, final) with the exception `unserialize()`
//! throws for it in PHP 8.4. The `eval()` of
//! `testInstancesAreNotCloned()` is a class declared at run time, whose
//! blank instance takes dynamic properties.
//!
//! Upstream: doctrine/instantiator tests/DoctrineTest/InstantiatorTest/InstantiatorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use std::any::Any;
use std::sync::{Arc, Mutex, PoisonError};

use php_value::compare::loose_equals;
use php_value::{Key, PhpArray, PhpObject, Value};
use php2rust_doctrine_instantiator::exception::{
    InstantiatorError, InvalidArgumentException, UnexpectedValueException,
};
use php2rust_doctrine_instantiator::{Instantiator, InstantiatorInterface};
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, is_a, register_class};

const SELF_CLASS: &str = "DoctrineTest\\InstantiatorTest\\InstantiatorTest";
const NAMESPACE: &str = "DoctrineTest\\InstantiatorTest";

fn set_up() -> Instantiator {
    support::declare_assets();
    register_class(ClassInfo::class(SELF_CLASS));
    Instantiator::new()
}

/// Provides a list of instantiable classes (existing)
fn get_instantiable_classes() -> Vec<String> {
    let asset = support::asset;
    vec![
        "stdClass".to_owned(),
        SELF_CLASS.to_owned(),
        "Doctrine\\Instantiator\\Instantiator".to_owned(),
        "Exception".to_owned(),
        "PharException".to_owned(),
        asset("SimpleSerializableAsset"),
        asset("ExceptionAsset"),
        asset("FinalExceptionAsset"),
        asset("PharExceptionAsset"),
        asset("UnCloneableAsset"),
        asset("XMLReaderAsset"),
        "PharException".to_owned(),
        "ArrayObject".to_owned(),
        asset("ArrayObjectAsset"),
        asset("SerializableArrayObjectAsset"),
        asset("WakeUpNoticesAsset"),
        asset("UnserializeExceptionArrayObjectAsset"),
        asset("SerializableFinalInternalChildAsset"),
    ]
}

/// Provides a list of instantiable classes (existing)
fn get_invalid_class_names() -> Vec<(&'static str, String)> {
    vec![
        (
            "invalid string",
            format!("{SELF_CLASS}{}", support::uniqid()),
        ),
        (
            "interface",
            "Doctrine\\Instantiator\\InstantiatorInterface".to_owned(),
        ),
        ("abstract class", support::asset("AbstractClassAsset")),
        ("trait", support::asset("SimpleTraitAsset")),
        ("enum", support::asset("SimpleEnumAsset")),
    ]
}

#[test]
fn testCanInstantiate() {
    for class_name in get_instantiable_classes() {
        let instantiator = set_up();
        let instance = instantiator.instantiate(&class_name).unwrap();
        assert!(is_a(instance.class(), &class_name), "{class_name}");
    }
}

#[test]
fn testInstantiatesSeparateInstances() {
    for class_name in get_instantiable_classes() {
        let instantiator = set_up();
        let instance1 = instantiator.instantiate(&class_name).unwrap();
        let instance2 = instantiator.instantiate(&class_name).unwrap();

        assert!(!Arc::ptr_eq(&instance1, &instance2), "{class_name}");
        assert!(
            loose_equals(&Value::Object(instance1), &Value::Object(instance2)),
            "{class_name}"
        );
    }
}

#[test]
fn testExceptionOnUnSerializationException() {
    let instantiator = set_up();
    let error = instantiator.instantiate("PDORow").unwrap_err();

    assert!(matches!(error, InstantiatorError::UnexpectedValue(_)));
    assert!(is_a(
        &error.to_thrown().class,
        UnexpectedValueException::CLASS
    ));
}

#[test]
fn testInstantiationFromNonExistingClass() {
    for (label, invalid_class_name) in get_invalid_class_names() {
        let instantiator = set_up();
        let error = instantiator.instantiate(&invalid_class_name).unwrap_err();

        assert!(
            matches!(error, InstantiatorError::InvalidArgument(_)),
            "{label}"
        );
        assert!(
            is_a(&error.to_thrown().class, InvalidArgumentException::CLASS),
            "{label}"
        );
    }
}

/// An object of a `#[\AllowDynamicProperties]` class: its properties can
/// be set after it is made.
#[derive(Debug)]
struct DynamicObject {
    class: String,
    properties: Mutex<PhpArray>,
}

impl PhpObject for DynamicObject {
    fn class(&self) -> &str {
        &self.class
    }

    fn properties(&self) -> PhpArray {
        self.properties
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[test]
fn testInstancesAreNotCloned() {
    let instantiator = set_up();
    let class_name = format!("TemporaryClass{}", support::uniqid());

    // eval("namespace $namespace; #[\AllowDynamicProperties] class $className {}")
    let class_name_with_namespace = format!("{NAMESPACE}\\{class_name}");
    let declared = class_name_with_namespace.clone();
    register_class(
        ClassInfo::class(class_name_with_namespace.clone()).new_instance(Arc::new(move || {
            Ok(Arc::new(DynamicObject {
                class: declared.clone(),
                properties: Mutex::new(PhpArray::new()),
            }) as php_value::Object)
        })),
    );

    let instance = instantiator
        .instantiate(&class_name_with_namespace)
        .unwrap();

    (instance.as_ref() as &dyn Any)
        .downcast_ref::<DynamicObject>()
        .unwrap()
        .properties
        .lock()
        .unwrap()
        .insert(Key::from("foo"), Value::from("bar"));

    let instance2 = instantiator
        .instantiate(&class_name_with_namespace)
        .unwrap();

    assert!(!instance2.properties().contains_key(&Key::from("foo")));
}
