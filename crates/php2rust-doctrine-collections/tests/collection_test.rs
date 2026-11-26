//! Port of tests/CollectionTest.php (with the CollectionTestCase tests it
//! inherits) at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d.
//!
//! Upstream: doctrine/collections tests/CollectionTest.php

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
use common::{TestObject, TestObjectPrivatePropertyOnly, assert_equals, foo_of, k};
use php_value::object::Closure;
use php_value::{Value, php_array};
use php2rust_doctrine_collections::expr::Value as ValueExpression;
use php2rust_doctrine_collections::{ArrayCollection, Collection, Criteria, Order, SortDirection};

/// `setUp()`: `new ArrayCollection()`.
fn set_up() -> Box<dyn Collection> {
    Box::new(ArrayCollection::default())
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

#[test]
fn testToString() {
    let mut collection = ArrayCollection::default();
    collection.add(Value::from("testing")).unwrap();
    let text = collection.to_string();
    assert!(text.starts_with("Doctrine\\Common\\Collections\\ArrayCollection@"));
}

#[test]
fn testMatching() {
    let mut collection = ArrayCollection::default();
    collection
        .offset_set(None, TestObjectPrivatePropertyOnly::object(42))
        .unwrap();
    collection
        .offset_set(None, TestObjectPrivatePropertyOnly::object(84))
        .unwrap();

    let col = collection
        .matching(&Criteria::new(
            Some(Criteria::expr().eq("fooBar", 42).into()),
            None,
            0,
            None,
        ))
        .unwrap();
    // A new collection (`assertNotSame`), of one element.
    assert_eq!(1, col.count());
    assert_eq!(2, collection.count());
}

#[test]
fn testMatchingCallable() {
    let mut collection = ArrayCollection::default();
    collection_test_case::fill_matching_fixture(&mut collection);
    common::set_foo_of(&collection.offset_get(&k(0)), 1);

    let col = collection
        .matching(&Criteria::new(
            Some(
                ValueExpression::new(Closure::object(|arguments| {
                    Value::Bool(foo_of(&arguments[0]) == Value::Int(1))
                }))
                .into(),
            ),
            None,
            0,
            None,
        ))
        .unwrap();

    assert_eq!(1, col.count());
    assert_eq!(2, collection.count());
}

#[test]
fn testMatchingOrdering() {
    let mut collection = ArrayCollection::default();
    let obj1 = TestObjectPrivatePropertyOnly::object(18);
    let obj2 = TestObjectPrivatePropertyOnly::object(10);
    let obj3 = TestObjectPrivatePropertyOnly::object(78);
    collection.set("one", obj1.clone());
    collection.set("two", obj2.clone());
    collection.set("three", obj3.clone());

    let mut criteria = Criteria::create();
    criteria.order_by([("fooBar", SortDirection::Ascending)]);

    let col = collection.matching(&criteria).unwrap();

    assert_equals(
        &Value::Array(php_array!["two" => obj2, "one" => obj1, "three" => obj3]),
        &Value::Array(col.to_array()),
    );
    assert_eq!(3, collection.count());
}

#[test]
fn testMatchingOrderingLegacy() {
    let mut collection = ArrayCollection::default();
    collection_test_case::fill_matching_fixture(&mut collection);

    let mut col = collection
        .matching(&Criteria::new(
            None,
            Some(vec![("foo".to_owned(), SortDirection::Descending.into())]),
            0,
            None,
        ))
        .unwrap();

    assert_eq!(2, col.count());
    assert_equals(&Value::from("baz"), &foo_of(&col.first()));
    assert_equals(&Value::from("bar"), &foo_of(&col.last()));
}

#[test]
fn testMatchingSlice() {
    let mut collection = ArrayCollection::default();
    collection_test_case::fill_matching_fixture(&mut collection);

    let col = collection
        .matching(&Criteria::new(None, None, 1, Some(1)))
        .unwrap();

    assert_eq!(1, col.count());
    assert_equals(&Value::from("baz"), &foo_of(&col.offset_get(&k(1))));
}

#[test]
fn testMatchingOrderingMixedOrderTypes() {
    let mut collection = ArrayCollection::default();
    let obj1 = TestObject::new("c", 1);
    collection
        .offset_set(None, Value::Object(obj1.clone()))
        .unwrap();

    let obj2 = TestObject::new("a", 2);
    collection
        .offset_set(None, Value::Object(obj2.clone()))
        .unwrap();

    let obj3 = TestObject::new("b", 3);
    collection.offset_set(None, Value::Object(obj3)).unwrap();

    let mut criteria = Criteria::create();
    criteria.order_by([
        (
            "foo",
            php2rust_doctrine_collections::Direction::from(Order::Ascending),
        ),
        ("bar", SortDirection::Descending.into()),
    ]);

    let mut col = collection.matching(&criteria).unwrap();

    assert_eq!(3, col.count());
    assert_eq!(Value::Object(obj2), col.first());
    assert_eq!(Value::Object(obj1), col.last());
}
