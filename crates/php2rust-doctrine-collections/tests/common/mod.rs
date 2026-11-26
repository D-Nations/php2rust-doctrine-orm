//! The upstream test fixtures (`TestObject`, `TestObjectPrivatePropertyOnly`,
//! `TestObjectPropertyHook`, and
//! `TestObjectWithPrivatePropertyInParentClass` of
//! ClosureExpressionVisitorTest.php) as PHP objects, and helpers for
//! PHPUnit's assertions.
//!
//! Upstream: doctrine/collections tests/TestObject.php tests/TestObjectPrivatePropertyOnly.php tests/TestObjectPropertyHook.php

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::disallowed_names,
    reason = "shared test helpers with upstream's names; a panic is a test failure"
)]

use std::sync::{Arc, Mutex};

use php_value::{Key, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::reflection::{RawProperties, register_raw_properties};
use php2rust_doctrine_collections::deprecations;

/// `Doctrine\Tests\Common\Collections\TestObject`: four public mixed
/// properties.
#[derive(Debug)]
pub struct TestObject {
    fields: Mutex<[Value; 4]>,
}

const FIELDS: [&str; 4] = ["foo", "bar", "baz", "qux"];

impl TestObject {
    pub const CLASS: &'static str = "Doctrine\\Tests\\Common\\Collections\\TestObject";

    /// `new TestObject($foo, $bar)`.
    pub fn new(foo: impl Into<Value>, bar: impl Into<Value>) -> Arc<Self> {
        Arc::new(Self {
            fields: Mutex::new([foo.into(), bar.into(), Value::Null, Value::Null]),
        })
    }

    /// `new TestObject($foo)` as a value.
    pub fn object(foo: impl Into<Value>) -> Value {
        Value::Object(Self::new(foo, Value::Null))
    }

    /// `new TestObject($foo, $bar)` as a value.
    pub fn object2(foo: impl Into<Value>, bar: impl Into<Value>) -> Value {
        Value::Object(Self::new(foo, bar))
    }

    pub fn foo(&self) -> Value {
        self.fields.lock().unwrap()[0].clone()
    }

    pub fn bar(&self) -> Value {
        self.fields.lock().unwrap()[1].clone()
    }

    pub fn set_foo(&self, value: impl Into<Value>) {
        self.fields.lock().unwrap()[0] = value.into();
    }

    pub fn set_bar(&self, value: impl Into<Value>) {
        self.fields.lock().unwrap()[1] = value.into();
    }
}

impl PhpObject for TestObject {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        let fields = self.fields.lock().unwrap();
        let mut properties = PhpArray::new();
        for (name, value) in FIELDS.iter().zip(fields.iter()) {
            properties.insert(*name, value.clone());
        }
        properties
    }
}

/// `$value->foo` of a `TestObject`.
pub fn foo_of(value: &Value) -> Value {
    value
        .as_object()
        .and_then(|o| o.downcast_ref::<TestObject>())
        .map(TestObject::foo)
        .unwrap()
}

/// `$value->foo = $foo` on a `TestObject`.
pub fn set_foo_of(value: &Value, foo: impl Into<Value>) {
    value
        .as_object()
        .and_then(|o| o.downcast_ref::<TestObject>())
        .unwrap()
        .set_foo(foo);
}

/// `$value->bar` of a `TestObject`.
pub fn bar_of(value: &Value) -> Value {
    value
        .as_object()
        .and_then(|o| o.downcast_ref::<TestObject>())
        .map(TestObject::bar)
        .unwrap()
}

/// `Doctrine\Tests\Common\Collections\TestObjectPrivatePropertyOnly`: one
/// private property, `fooBar`.
#[derive(Debug)]
pub struct TestObjectPrivatePropertyOnly {
    foo_bar: Value,
}

impl TestObjectPrivatePropertyOnly {
    pub const CLASS: &'static str =
        "Doctrine\\Tests\\Common\\Collections\\TestObjectPrivatePropertyOnly";

    /// `new TestObjectPrivatePropertyOnly($fooBar)` as a value.
    pub fn object(foo_bar: impl Into<Value>) -> Value {
        register_raw_properties::<Self>();
        Value::Object(Arc::new(Self {
            foo_bar: foo_bar.into(),
        }))
    }
}

impl PhpObject for TestObjectPrivatePropertyOnly {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }
}

impl RawProperties for TestObjectPrivatePropertyOnly {
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>> {
        (name == "fooBar").then(|| Ok(self.foo_bar.clone()))
    }
}

/// `TestObjectWithPrivatePropertyInParentClass extends
/// TestObjectPrivatePropertyOnly` (ClosureExpressionVisitorTest.php).
#[derive(Debug)]
pub struct TestObjectWithPrivatePropertyInParentClass {
    parent: TestObjectPrivatePropertyOnly,
}

impl TestObjectWithPrivatePropertyInParentClass {
    pub const CLASS: &'static str =
        "Doctrine\\Tests\\Common\\Collections\\TestObjectWithPrivatePropertyInParentClass";

    pub fn object(foo_bar: impl Into<Value>) -> Value {
        register_raw_properties::<Self>();
        Value::Object(Arc::new(Self {
            parent: TestObjectPrivatePropertyOnly {
                foo_bar: foo_bar.into(),
            },
        }))
    }
}

impl PhpObject for TestObjectWithPrivatePropertyInParentClass {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn instance_of(&self, class: &str) -> bool {
        class == TestObjectPrivatePropertyOnly::CLASS
    }
}

impl RawProperties for TestObjectWithPrivatePropertyInParentClass {
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>> {
        self.parent.raw_property(name)
    }
}

/// `Doctrine\Tests\Common\Collections\TestObjectPropertyHook`: a public
/// `int|null $fooBar` whose `get` hook returns `-$this->fooBar`.
#[derive(Debug, Default)]
pub struct TestObjectPropertyHook {
    foo_bar: Mutex<Option<i64>>,
}

impl TestObjectPropertyHook {
    pub const CLASS: &'static str = "Doctrine\\Tests\\Common\\Collections\\TestObjectPropertyHook";

    pub fn new() -> Arc<Self> {
        register_raw_properties::<Self>();
        Arc::new(Self::default())
    }

    /// `$object->fooBar = $value`.
    pub fn set_foo_bar(&self, value: i64) {
        *self.foo_bar.lock().unwrap() = Some(value);
    }
}

impl PhpObject for TestObjectPropertyHook {
    fn class(&self) -> &str {
        Self::CLASS
    }

    /// Read through the hook: `-$this->fooBar` (`-null` is `0`).
    fn properties(&self) -> PhpArray {
        let raw = *self.foo_bar.lock().unwrap();
        let mut properties = PhpArray::new();
        properties.insert("fooBar", Value::Int(raw.map_or(0, i64::wrapping_neg)));
        properties
    }
}

impl RawProperties for TestObjectPropertyHook {
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>> {
        (name == "fooBar").then(|| Ok(self.foo_bar.lock().unwrap().map_or(Value::Null, Value::Int)))
    }
}

/// An int or string key.
pub fn k(key: impl Into<Key>) -> Key {
    key.into()
}

/// PHPUnit's `assertEquals()`.
pub fn assert_equals(expected: &Value, actual: &Value) {
    assert!(
        php_value::vectors::phpunit_equals(expected, actual),
        "Failed asserting that {actual:?} matches expected {expected:?}."
    );
}

/// `expectException($class)` with `expectExceptionMessage($message)`.
pub fn assert_thrown<T: std::fmt::Debug>(
    result: Result<T, Thrown>,
    class: &str,
    message: Option<&str>,
) {
    let error = result.expect_err("Failed asserting that exception is thrown");
    assert_eq!(error.class, class);
    if let Some(message) = message {
        assert_eq!(error.message.to_string_lossy(), message);
    }
}

/// `$this->expectDeprecationWithIdentifier($link)` around `f`.
pub fn expect_deprecation<T>(link: &str, f: impl FnOnce() -> T) -> T {
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(link);
    let result = f();
    assert!(
        deprecations::count(link) > before,
        "Expected deprecation with identifier \"{link}\" was not triggered"
    );
    deprecations::disable();
    result
}

/// `usort($objects, $sort)`: a stable insertion sort.
pub fn usort(
    items: &mut [Value],
    sort: &dyn Fn(&Value, &Value) -> Result<i64, Thrown>,
) -> Result<(), Thrown> {
    for i in 1..items.len() {
        let mut j = i;
        while j > 0 && sort(&items[j - 1], &items[j])? > 0 {
            items.swap(j - 1, j);
            j -= 1;
        }
    }
    Ok(())
}
