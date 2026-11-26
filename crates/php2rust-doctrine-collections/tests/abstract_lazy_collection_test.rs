//! Port of tests/AbstractLazyCollectionTest.php (with the
//! CollectionTestCase tests it inherits) at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d.
//!
//! Upstream: doctrine/collections tests/AbstractLazyCollectionTest.php

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

use common::collection_test_case;
use common::collection_test_case::is_numeric;
use common::{
    LazyArrayCollection, TestObjectPrivatePropertyOnly, assert_equals, k, lazy_array_collection,
};
use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_collections::{Collection, Criteria, ReadableCollection, Selectable};

/// `setUp()`: `new LazyArrayCollection(new ArrayCollection())`.
fn set_up() -> Box<dyn Collection> {
    Box::new(lazy_array_collection(PhpArray::new()))
}

// The tests of `CollectionTestCase`.

#[test]
fn testIssetAndUnset() {
    collection_test_case::testIssetAndUnset(set_up);
}

#[test]
fn testRemovingNonExistentEntryReturnsNull() {
    collection_test_case::testRemovingNonExistentEntryReturnsNull(set_up);
}

#[test]
fn testExists() {
    collection_test_case::testExists(set_up);
}

#[test]
fn testFindFirst() {
    collection_test_case::testFindFirst(set_up);
}

#[test]
fn testFindFirstNotFound() {
    collection_test_case::testFindFirstNotFound(set_up);
}

#[test]
fn testMap() {
    collection_test_case::testMap(set_up);
}

#[test]
fn testReduce() {
    collection_test_case::testReduce(set_up);
}

#[test]
fn testFilter() {
    collection_test_case::testFilter(set_up);
}

#[test]
fn testFilterByValueAndKey() {
    collection_test_case::testFilterByValueAndKey(set_up);
}

#[test]
fn testFirstAndLast() {
    collection_test_case::testFirstAndLast(set_up);
}

#[test]
fn testArrayAccess() {
    collection_test_case::testArrayAccess(set_up);
}

#[test]
fn testContainsKey() {
    collection_test_case::testContainsKey(set_up);
}

#[test]
fn testContains() {
    collection_test_case::testContains(set_up);
}

#[test]
fn testSearch() {
    collection_test_case::testSearch(set_up);
}

#[test]
fn testGet() {
    collection_test_case::testGet(set_up);
}

#[test]
fn testGetKeys() {
    collection_test_case::testGetKeys(set_up);
}

#[test]
fn testGetValues() {
    collection_test_case::testGetValues(set_up);
}

#[test]
fn testCount() {
    collection_test_case::testCount(set_up);
}

#[test]
fn testForAll() {
    collection_test_case::testForAll(set_up);
}

#[test]
fn testPartition() {
    collection_test_case::testPartition(set_up);
}

#[test]
fn testClear() {
    collection_test_case::testClear(set_up);
}

#[test]
fn testRemove() {
    collection_test_case::testRemove(set_up);
}

#[test]
fn testRemoveElement() {
    collection_test_case::testRemoveElement(set_up);
}

#[test]
fn testSlice() {
    collection_test_case::testSlice(set_up);
}

#[test]
fn testCanRemoveNullValuesByKey() {
    collection_test_case::testCanRemoveNullValuesByKey(set_up);
}

#[test]
fn testCanVerifyExistingKeysWithNullValues() {
    collection_test_case::testCanVerifyExistingKeysWithNullValues(set_up);
}

#[test]
fn testMatchingAlwaysReturnsCollection() {
    collection_test_case::testMatchingAlwaysReturnsCollection(set_up);
}

/// `buildCollection($elements)`.
fn build_collection(elements: PhpArray) -> LazyArrayCollection {
    lazy_array_collection(elements)
}

#[test]
fn testClearInitializes() {
    let mut collection = build_collection(php_array!["a", "b", "c"]);

    collection.clear().unwrap();

    assert!(collection.is_initialized());
    assert_eq!(0, collection.count().unwrap());
}

#[test]
fn testFilterInitializes() {
    let mut collection = build_collection(php_array![1, "foo", 3]);

    let mut res = collection
        .filter(&mut |value, _| is_numeric(value))
        .unwrap();

    assert_equals(
        &Value::Array(php_array![0 => 1, 2 => 3]),
        &Value::Array(res.to_array().unwrap()),
    );
}

#[test]
fn testForAllInitializes() {
    let mut collection = build_collection(php_array!["foo", "bar"]);

    assert!(
        collection
            .for_all(&mut |_, e| matches!(e, Value::String(_)))
            .unwrap()
    );

    assert!(
        !collection
            .for_all(&mut |_, e| matches!(e, Value::Array(_)))
            .unwrap()
    );
}

#[test]
fn testMapInitializes() {
    let mut collection = build_collection(php_array![1, 2]);

    let mut res = collection
        .map(&mut |e| php_value::ops::mul(e, &Value::Int(2)).unwrap())
        .unwrap();
    assert_equals(
        &Value::Array(php_array![2, 4]),
        &Value::Array(res.to_array().unwrap()),
    );
}

#[test]
fn testPartitionInitializes() {
    let mut collection = build_collection(php_array![true, false]);
    let (mut matches, mut no_matches) = collection
        .partition(&mut |_, e| *e == Value::Bool(true))
        .unwrap();
    assert_equals(&matches.offset_get(&k(0)).unwrap(), &Value::Bool(true));
    assert_equals(&no_matches.offset_get(&k(0)).unwrap(), &Value::Bool(false));
}

#[test]
fn testSliceInitializes() {
    let mut collection = build_collection(php_array!["one", "two", "three"]);

    let slice = collection.slice(0, Some(1)).unwrap();
    assert_equals(&Value::Array(php_array!["one"]), &Value::Array(slice));

    let slice = collection.slice(1, None).unwrap();
    assert_equals(
        &Value::Array(php_array![1 => "two", 2 => "three"]),
        &Value::Array(slice),
    );

    let slice = collection.slice(1, Some(1)).unwrap();
    assert_equals(&Value::Array(php_array![1 => "two"]), &Value::Array(slice));
}

#[test]
fn testGetInitializes() {
    let value = Value::from("foo");
    let mut collection = build_collection(php_array![value.clone()]);
    assert_eq!(value, collection.offset_get(&k(0)).unwrap());
}

#[test]
fn testUnsetInitializes() {
    let mut collection = build_collection(php_array!["foo", "bar"]);

    collection.offset_unset(&k(0)).unwrap();
    assert_eq!(1, collection.count().unwrap());
    assert!(!collection.offset_exists(&k(0)).unwrap());
}

#[test]
fn testMatchingInitializes() {
    let mut collection = build_collection(php_array!["foo", "bar", "baz"]);

    assert!(!collection.is_initialized());

    let mut result = collection.matching(&Criteria::create()).unwrap();

    assert!(collection.is_initialized());
    assert_eq!(3, result.count().unwrap());
}

#[test]
fn testMatchingWithCriteria() {
    let obj1 = TestObjectPrivatePropertyOnly::object("bar");
    let obj2 = TestObjectPrivatePropertyOnly::object("baz");
    let obj3 = TestObjectPrivatePropertyOnly::object("bar");

    let mut collection = build_collection(php_array![obj1.clone(), obj2, obj3]);

    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq("fooBar", "bar"));

    let mut result = collection.matching(&criteria).unwrap();

    assert_eq!(2, result.count().unwrap());
    assert_eq!(obj1, result.first().unwrap());
}
