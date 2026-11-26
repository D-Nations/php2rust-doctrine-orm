//! UnexpectedValueExceptionTest.php at doctrine/instantiator
//! 23da848e1a2308728fe5fdddabf4be17ff9720c7. Names and assertions are
//! PHPUnit's. `new ReflectionClass($this)` reflects the test class, which
//! is declared in the class registry here; a thrown object is a
//! `php_value::Thrown` (class, message and code), so `assertSame()` on
//! the previous exception compares those.
//!
//! Upstream: doctrine/instantiator tests/DoctrineTest/InstantiatorTest/Exception/UnexpectedValueExceptionTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

mod support;

use php_value::Thrown;
use php2rust_doctrine_instantiator::exception::{ExceptionInterface, UnexpectedValueException};
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, ReflectionClass, is_a, register_class,
};

const SELF_CLASS: &str = "DoctrineTest\\InstantiatorTest\\Exception\\UnexpectedValueExceptionTest";

#[test]
fn testFromSerializationTriggeredException() {
    support::declare_assets();
    register_class(ClassInfo::class(SELF_CLASS));
    let reflection_class = ReflectionClass::new(SELF_CLASS).unwrap();
    let previous = Thrown::new("Exception", "", 0);
    let exception = UnexpectedValueException::from_serialization_triggered_exception(
        &reflection_class,
        previous.clone(),
    );

    assert_eq!(Some(&previous), exception.get_previous());
    assert_eq!(
        format!(
            "An exception was raised while trying to instantiate an instance of \"{SELF_CLASS}\" via un-serialization"
        ),
        exception.get_message()
    );
}

#[test]
fn testFromUncleanUnSerialization() {
    support::declare_assets();
    let reflection = ReflectionClass::new(&support::asset("AbstractClassAsset")).unwrap();
    let exception = UnexpectedValueException::from_unclean_un_serialization(
        &reflection,
        "foo",
        123,
        "bar",
        456,
    );

    assert_eq!(
        format!(
            "Could not produce an instance of \"{}\" via un-serialization, since an error was triggered in file \"bar\" at line \"456\"",
            support::asset("AbstractClassAsset")
        ),
        exception.get_message()
    );

    let previous = exception.get_previous().unwrap();
    assert!(is_a(&previous.class, "Exception"));
    assert_eq!(b"foo", previous.message.as_bytes());
    assert_eq!(123, previous.code);
}
