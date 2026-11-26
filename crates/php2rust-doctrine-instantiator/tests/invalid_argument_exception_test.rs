//! InvalidArgumentExceptionTest.php at doctrine/instantiator
//! 23da848e1a2308728fe5fdddabf4be17ff9720c7. Names and assertions are
//! PHPUnit's; the assets are declared in the class registry
//! (`support::declare_assets()`).
//!
//! Upstream: doctrine/instantiator tests/DoctrineTest/InstantiatorTest/Exception/InvalidArgumentExceptionTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use php2rust_doctrine_instantiator::exception::{ExceptionInterface, InvalidArgumentException};
use php2rust_doctrine_persistence::php_reflection::ReflectionClass;

const SELF_CLASS: &str = "DoctrineTest\\InstantiatorTest\\Exception\\InvalidArgumentExceptionTest";

#[test]
fn testFromNonExistingTypeWithNonExistingClass() {
    support::declare_assets();
    let class_name = format!("{SELF_CLASS}{}", support::uniqid());
    let exception = InvalidArgumentException::from_non_existing_class(&class_name);

    assert_eq!(
        format!("The provided class \"{class_name}\" does not exist"),
        exception.get_message()
    );
}

#[test]
fn testFromNonExistingTypeWithTrait() {
    support::declare_assets();
    let trait_name = support::asset("SimpleTraitAsset");
    let exception = InvalidArgumentException::from_non_existing_class(&trait_name);

    assert_eq!(
        format!("The provided type \"{trait_name}\" is a trait, and cannot be instantiated"),
        exception.get_message()
    );
}

#[test]
fn testFromNonExistingTypeWithInterface() {
    support::declare_assets();
    let interface = "Doctrine\\Instantiator\\InstantiatorInterface";
    let exception = InvalidArgumentException::from_non_existing_class(interface);

    assert_eq!(
        format!("The provided type \"{interface}\" is an interface, and cannot be instantiated"),
        exception.get_message()
    );
}

#[test]
fn testFromAbstractClass() {
    support::declare_assets();
    let reflection = ReflectionClass::new(&support::asset("AbstractClassAsset")).unwrap();
    let exception = InvalidArgumentException::from_abstract_class(&reflection);

    assert_eq!(
        format!(
            "The provided class \"{}\" is abstract, and cannot be instantiated",
            support::asset("AbstractClassAsset")
        ),
        exception.get_message()
    );
}
