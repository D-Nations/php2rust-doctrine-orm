//! Port of tests/Tests/ORM/Tools/DebugTest.php.
//!
//! The `TestAsset` classes are objects whose `(array)` cast is PHP's: the
//! property table in declaration order (the parent's first), private
//! and protected properties under their mangled keys, and a private
//! property of a parent kept beside the child's of the same name.
//! `print_r()` is a helper of this file for arrays and such objects.
//! `php_value` has no named time zones: `new
//! DateTimeZone('Europe/Rome')` is its offset on 2010-10-10 (+02:00), so
//! the exported `date` is the same. `testExportObjectWithReference`'s
//! second assertion is about PHP references (`$bar['foo']` changes with
//! `$baz->foo`), which Rust values do not have: the port checks that the
//! export is a copy.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/DebugTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::date_create::date_create;
use php_value::datetime::{DateTime, TimeZone};
use php_value::object::{ArrayIterator, ArrayObject, StdClass};
use php_value::{Key, PhpArray, PhpObject, Value};
use php2rust_doctrine_orm::collection_object::{self, ArrayCollectionObject};
use php2rust_doctrine_orm::tools::debug::export;

const TEST_ASSET: &str = "Doctrine\\Tests\\ORM\\Tools\\TestAsset\\";

/// `$var->$name` of an exported object.
fn prop(var: &Value, name: &str) -> Value {
    let Value::Object(object) = var else {
        panic!("not an object: {var:?}");
    };
    object
        .properties()
        .get(&Key::from(name))
        .cloned()
        .unwrap_or(Value::Null)
}

fn s(text: &str) -> Value {
    Value::string(text)
}

fn date(text: &str, zone: TimeZone) -> php_value::datetime::DateTimeImmutable {
    date_create(text.as_bytes(), Some(zone)).unwrap().unwrap()
}

#[tokio::test]
async fn testExportObject() {
    let mut properties = PhpArray::new();
    properties.insert("foo", s("bar"));
    properties.insert("bar", Value::Int(1234));
    let obj = StdClass::object(properties);

    let var = export(&obj, 2).await.unwrap();
    assert_eq!(prop(&var, "__CLASS__"), s("stdClass"));
}

#[tokio::test]
async fn testExportObjectWithReference() {
    let foo = s("bar");
    let mut bar = PhpArray::new();
    bar.insert("foo", foo);
    let baz = StdClass::object(bar.clone());

    let var = export(&baz, 2).await.unwrap();
    // `$baz->foo = 'tab'`: a new object, as Rust's `stdClass` is a value.
    let mut changed = bar.clone();
    changed.insert("foo", s("tab"));
    let _baz = StdClass::object(changed);

    assert_eq!(prop(&var, "foo"), s("bar"));
}

#[tokio::test]
async fn testExportArray() {
    let d = PhpArray::list([s("e"), s("f")]);
    let mut b = PhpArray::list([s("c")]);
    b.insert("d", Value::Array(d));
    let mut array = PhpArray::new();
    array.insert("a", s("b"));
    array.insert("b", Value::Array(b.clone()));
    let var = export(&Value::Array(array.clone()), 2).await.unwrap();
    let mut expected = array;
    b.insert("d", s("Array(2)"));
    expected.insert("b", Value::Array(b));
    assert_eq!(var, Value::Array(expected));
}

#[tokio::test]
async fn testExportDateTime() {
    let obj = DateTime::object(date("2010-10-10 10:10:10", TimeZone::Utc));

    let var = export(&obj, 2).await.unwrap();
    assert_eq!(prop(&var, "__CLASS__"), s("DateTime"));
    assert_eq!(prop(&var, "date"), s("2010-10-10T10:10:10+00:00"));
}

#[tokio::test]
async fn testExportDateTimeImmutable() {
    let obj = date("2010-10-10 10:10:10", TimeZone::Utc).object();

    let var = export(&obj, 2).await.unwrap();
    assert_eq!(prop(&var, "__CLASS__"), s("DateTimeImmutable"));
    assert_eq!(prop(&var, "date"), s("2010-10-10T10:10:10+00:00"));
}

#[tokio::test]
async fn testExportDateTimeZone() {
    // `new DateTimeZone('Europe/Rome')`, CEST on that day.
    let obj = date("2010-10-10 12:34:56", TimeZone::Offset(7200)).object();

    let var = export(&obj, 2).await.unwrap();
    assert_eq!(prop(&var, "__CLASS__"), s("DateTimeImmutable"));
    assert_eq!(prop(&var, "date"), s("2010-10-10T12:34:56+02:00"));
}

/// `assertContains($needle, $haystack)`.
fn assert_contains(needle: &Value, haystack: &Value) {
    let Value::Array(haystack) = haystack else {
        panic!("not an array: {haystack:?}");
    };
    assert!(haystack.values().any(|v| v == needle), "{haystack:?}");
}

#[tokio::test]
async fn testExportArrayTraversable() {
    let obj: Value = Value::Object(Arc::new(ArrayObject::new(PhpArray::list([s("foobar")]))));

    let var = export(&obj, 2).await.unwrap();
    assert_contains(&s("foobar"), &prop(&var, "__STORAGE__"));

    let it: Value = Value::Object(Arc::new(ArrayIterator::new(PhpArray::list([s("foobar")]))));

    let var = export(&it, 5).await.unwrap();
    assert_contains(&s("foobar"), &prop(&var, "__STORAGE__"));
}

/// A `TestAsset` object: its class and its property table, `(array)`
/// keys and values.
#[derive(Debug)]
struct TestAssetObject {
    class: String,
    table: PhpArray,
}

impl PhpObject for TestAssetObject {
    fn class(&self) -> &str {
        &self.class
    }

    fn properties(&self) -> PhpArray {
        self.table.clone()
    }
}

fn protected(name: &str) -> Key {
    Key::from_bytes(format!("\0*\0{name}").as_bytes())
}

fn private(class: &str, name: &str) -> Key {
    Key::from_bytes(format!("\0{TEST_ASSET}{class}\0{name}").as_bytes())
}

/// `new TestAsset\ChildClass()`.
fn child_class() -> Value {
    let mut table = PhpArray::new();
    table.insert("parentPublicAttribute", Value::Int(1));
    table.insert(protected("parentProtectedAttribute"), Value::Int(2));
    table.insert(
        private("ParentClass", "parentPrivateAttribute"),
        Value::Int(3),
    );
    table.insert("childPublicAttribute", Value::Int(4));
    table.insert(protected("childProtectedAttribute"), Value::Int(5));
    table.insert(
        private("ChildClass", "childPrivateAttribute"),
        Value::Int(6),
    );
    Value::Object(Arc::new(TestAssetObject {
        class: format!("{TEST_ASSET}ChildClass"),
        table,
    }))
}

/// `new TestAsset\ChildWithSameAttributesClass()`.
fn child_with_same_attributes_class() -> Value {
    let mut table = PhpArray::new();
    table.insert("parentPublicAttribute", Value::Int(4));
    table.insert(protected("parentProtectedAttribute"), Value::Int(5));
    table.insert(
        private("ParentClass", "parentPrivateAttribute"),
        Value::Int(3),
    );
    table.insert(
        private("ChildWithSameAttributesClass", "parentPrivateAttribute"),
        Value::Int(6),
    );
    Value::Object(Arc::new(TestAssetObject {
        class: format!("{TEST_ASSET}ChildWithSameAttributesClass"),
        table,
    }))
}

/// `print_r()`'s name of a property key: `name:protected`,
/// `name:Class:private`.
fn property_name(key: &Key) -> String {
    let text = match key {
        Key::Int(i) => i.to_string(),
        Key::Str(s) => String::from_utf8_lossy(s.as_bytes()).into_owned(),
    };
    let parts: Vec<&str> = text.split('\0').collect();
    match parts.as_slice() {
        ["", "*", name] => format!("{name}:protected"),
        ["", class, name] => format!("{name}:{class}:private"),
        _ => text,
    }
}

/// `print_r($value, true)` of an int, an array or an object of ints.
fn print_r(value: &Value, indent: usize) -> String {
    let (head, entries): (String, Vec<(String, Value)>) = match value {
        Value::Int(i) => return i.to_string(),
        Value::Array(array) => (
            "Array".to_owned(),
            array
                .iter()
                .map(|(k, v)| (property_name(k), v.clone()))
                .collect(),
        ),
        Value::Object(object) => (
            format!("{} Object", object.class()),
            object
                .properties()
                .iter()
                .map(|(k, v)| (property_name(k), v.clone()))
                .collect(),
        ),
        other => panic!("not printed: {other:?}"),
    };
    let pad = " ".repeat(indent);
    let mut out = format!("{head}\n{pad}(\n");
    for (name, v) in entries {
        out.push_str(&format!(
            "{pad}    [{name}] => {}\n",
            print_r(&v, indent + 8)
        ));
    }
    out.push_str(&format!("{pad})\n"));
    if indent == 0 {
        out.push('\n');
    }
    out
}

fn expected(entries: &[(&str, i64)]) -> PhpArray {
    let mut array = PhpArray::new();
    for (key, value) in entries {
        array.insert(*key, Value::Int(*value));
    }
    array
}

/// `provideAttributesCases()`.
fn provide_attributes_cases() -> Vec<(&'static str, Value, PhpArray)> {
    let parent_private = format!("parentPrivateAttribute:{TEST_ASSET}ParentClass:private");
    let child_private = format!("childPrivateAttribute:{TEST_ASSET}ChildClass:private");
    let same_private =
        format!("parentPrivateAttribute:{TEST_ASSET}ChildWithSameAttributesClass:private");
    vec![
        (
            "different-attributes",
            child_class(),
            expected(&[
                ("parentPublicAttribute", 1),
                ("parentProtectedAttribute:protected", 2),
                (&parent_private, 3),
                ("childPublicAttribute", 4),
                ("childProtectedAttribute:protected", 5),
                (&child_private, 6),
            ]),
        ),
        (
            "same-attributes",
            child_with_same_attributes_class(),
            expected(&[
                ("parentPublicAttribute", 4),
                ("parentProtectedAttribute:protected", 5),
                (&parent_private, 3),
                (&same_private, 6),
            ]),
        ),
    ]
}

/// `assertSame()` of two arrays: the same keys in the same order, with
/// identical values.
fn assert_same_array(expected: &PhpArray, actual: &PhpArray) {
    let expected: Vec<_> = expected.iter().collect();
    let actual: Vec<_> = actual.iter().collect();
    assert_eq!(expected.len(), actual.len(), "{expected:?} != {actual:?}");
    for ((ek, ev), (ak, av)) in expected.iter().zip(&actual) {
        assert_eq!(ek, ak);
        assert!(ev.identical(av), "{ev:?} !== {av:?}");
    }
}

#[tokio::test]
async fn testExportParentAttributes() {
    for (case, class, expected) in provide_attributes_cases() {
        let actual_representation = print_r(&class, 0);
        let expected_representation = print_r(&Value::Array(expected.clone()), 0);

        let actual_representation =
            &actual_representation[actual_representation.find('(').unwrap()..];
        let expected_representation =
            &expected_representation[expected_representation.find('(').unwrap()..];

        assert_eq!(expected_representation, actual_representation, "{case}");

        let var = export(&class, 3).await.unwrap();
        let mut var = php_value::object::to_array(&var);
        var.remove(&Key::from("__CLASS__"));

        assert_same_array(&expected, &var);
    }
}

#[tokio::test]
async fn testCollectionsAreCastIntoArrays() {
    let collection = ArrayCollectionObject::value(PhpArray::new());
    collection_object::add(&collection, s("foo")).unwrap();
    collection_object::add(&collection, s("bar")).unwrap();

    let var = export(&collection, 2).await.unwrap();
    assert_eq!(var, Value::Array(PhpArray::list([s("foo"), s("bar")])));
}
