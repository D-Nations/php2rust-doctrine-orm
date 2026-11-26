//! Port of tests/RuntimeReflectionPropertyTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6. The proxy mock's `__get()`,
//! `__set()` and `__isset()` run its initializer, which counts the calls
//! (the `DummyMock` expectations): reflection must not reach them.
//!
//! Upstream: doctrine/persistence tests/RuntimeReflectionPropertyTest.php

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

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use common::{FixtureObject, FixtureProperty};
use php_value::{Key, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::reflection::RawProperties;
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, MethodInfo, PropertyInfo, ReflectionObject, register_class,
    register_reflection_object,
};
use php2rust_doctrine_persistence::reflection::RuntimeReflectionProperty;
use php2rust_doctrine_persistence::{Proxy, proxy};

const TEST_CLASS: &str = "Doctrine\\Tests\\Persistence\\RuntimeReflectionPropertyTestClass";
const PROXY_MOCK: &str = "Doctrine\\Tests\\Persistence\\RuntimeReflectionPropertyTestProxyMock";

fn register() {
    common::register_fixtures();
    register_class(
        ClassInfo::class(TEST_CLASS)
            .property(
                PropertyInfo::typed("test", "string", false)
                    .default_value(Value::from("testValue")),
            )
            .property(
                PropertyInfo::typed("privateTest", "string", false)
                    .private()
                    .default_value(Value::from("privateTestValue")),
            )
            .method(MethodInfo::new("getPrivateTest")),
    );
    register_class(
        ClassInfo::class(PROXY_MOCK)
            .implements(proxy::CLASS)
            .property(
                PropertyInfo::typed("initialized", "bool", false)
                    .private()
                    .default_value(Value::Bool(false)),
            )
            .property(
                PropertyInfo::typed("checkedProperty", "string", false)
                    .default_value(Value::from("testValue")),
            )
            .property(
                PropertyInfo::typed("initializer", "Closure", true)
                    .protected()
                    .default_value(Value::Null),
            ),
    );
    register_reflection_object::<RuntimeReflectionPropertyTestProxyMock>();
}

/// `new RuntimeReflectionPropertyTestClass()`.
fn new_test_class() -> FixtureObject {
    FixtureObject::new(
        TEST_CLASS,
        vec![
            FixtureProperty::typed("test", Some(Value::from("testValue"))),
            FixtureProperty::typed("privateTest", Some(Value::from("privateTestValue"))).private(),
        ],
    )
}

/// `RuntimeReflectionPropertyTestProxyMock`: a mock that simulates proxy
/// property lazy loading.
#[derive(Debug, Default)]
struct RuntimeReflectionPropertyTestProxyMock {
    initialized: AtomicBool,
    checked_property: Mutex<Option<Value>>,
    /// Calls of the initializer.
    initializer_calls: AtomicUsize,
}

impl RuntimeReflectionPropertyTestProxyMock {
    fn new() -> Self {
        Self {
            checked_property: Mutex::new(Some(Value::from("testValue"))),
            ..Self::default()
        }
    }

    fn run_initializer(&self) {
        if !self.initialized.load(Ordering::SeqCst) {
            self.initializer_calls.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// `$mockProxy->checkedProperty`: the declared property, or `__get()`
    /// once it is unset.
    fn checked_property(&self) -> Value {
        let value = self.checked_property.lock().unwrap().clone();
        value.unwrap_or_else(|| {
            self.run_initializer();
            Value::Null
        })
    }

    /// `unset($mockProxy->checkedProperty)`.
    fn unset_checked_property(&self) {
        *self.checked_property.lock().unwrap() = None;
    }
}

impl PhpObject for RuntimeReflectionPropertyTestProxyMock {
    fn class(&self) -> &str {
        PROXY_MOCK
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        if let Some(value) = self.checked_property.lock().unwrap().clone() {
            properties.insert(Key::from("checkedProperty"), value);
        }
        properties
    }
}

impl Proxy for RuntimeReflectionPropertyTestProxyMock {
    fn load(&self) -> Result<(), Thrown> {
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    fn set_initialized(&self, initialized: bool) -> Option<()> {
        self.initialized.store(initialized, Ordering::SeqCst);
        Some(())
    }
}

impl RawProperties for RuntimeReflectionPropertyTestProxyMock {
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>> {
        match name {
            "checkedProperty" => Some(self.checked_property.lock().unwrap().clone().ok_or_else(|| {
                Thrown::new(
                    "Error",
                    format!("Typed property {PROXY_MOCK}::$checkedProperty must not be accessed before initialization"),
                    0,
                )
            })),
            "initialized" => Some(Ok(Value::Bool(self.is_initialized()))),
            _ => None,
        }
    }
}

impl ReflectionObject for RuntimeReflectionPropertyTestProxyMock {
    fn set_raw_property(&self, name: &str, value: Value) -> Option<Result<(), Thrown>> {
        match name {
            "checkedProperty" => {
                // A declared property that was unset goes through __set().
                if self.checked_property.lock().unwrap().is_none() {
                    self.run_initializer();
                }
                *self.checked_property.lock().unwrap() = Some(value);
                Some(Ok(()))
            }
            _ => None,
        }
    }

    fn unset_raw_property(&self, name: &str) -> Option<Result<(), Thrown>> {
        match name {
            "checkedProperty" => {
                self.unset_checked_property();
                Some(Ok(()))
            }
            _ => None,
        }
    }

    fn as_proxy(&self) -> Option<&dyn Proxy> {
        Some(self)
    }
}

#[test]
fn testGetSetValue() {
    // #[TestWith(['test', 'testValue'])], #[TestWith(['privateTest', 'privateTestValue'])]
    for (name, value) in [("test", "testValue"), ("privateTest", "privateTestValue")] {
        register();
        let object = new_test_class();
        let refl_property = RuntimeReflectionProperty::new(TEST_CLASS, name).unwrap();

        assert!(
            refl_property
                .get_value(&object)
                .identical(&Value::from(value))
        );

        refl_property
            .set_value(&object, Value::from("changedValue"))
            .unwrap();

        assert!(
            refl_property
                .get_value(&object)
                .identical(&Value::from("changedValue"))
        );
    }
}

#[test]
fn testGetValueOnProxyProperty() {
    // #[TestWith] twice with the same proxy class.
    for proxy_class in [PROXY_MOCK, PROXY_MOCK] {
        register();
        let mock_proxy = RuntimeReflectionPropertyTestProxyMock::new();

        let refl_property = RuntimeReflectionProperty::new(proxy_class, "checkedProperty").unwrap();

        assert!(
            refl_property
                .get_value(&mock_proxy)
                .identical(&Value::from("testValue"))
        );
        mock_proxy.unset_checked_property();
        assert!(refl_property.get_value(&mock_proxy).is_null());

        // $getCheckMock->expects(self::never())->method('callGet')
        assert_eq!(mock_proxy.initializer_calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn testSetValueOnProxyProperty() {
    register();
    let mock_proxy = RuntimeReflectionPropertyTestProxyMock::new();
    let refl_property = RuntimeReflectionProperty::new(PROXY_MOCK, "checkedProperty").unwrap();

    refl_property
        .set_value(&mock_proxy, Value::from("newValue"))
        .unwrap();
    assert!(
        mock_proxy
            .checked_property()
            .identical(&Value::from("newValue"))
    );

    mock_proxy.unset_checked_property();
    refl_property
        .set_value(&mock_proxy, Value::from("otherNewValue"))
        .unwrap();
    assert!(
        mock_proxy
            .checked_property()
            .identical(&Value::from("otherNewValue"))
    );

    // $setCheckMock->expects(self::never())->method('callSet')
    assert_eq!(mock_proxy.initializer_calls.load(Ordering::SeqCst), 0);
}
