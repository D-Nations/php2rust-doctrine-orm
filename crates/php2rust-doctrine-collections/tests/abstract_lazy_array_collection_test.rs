//! Port of tests/AbstractLazyArrayCollectionTest.php (with the
//! ArrayCollectionTestCase tests it inherits) at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d.
//!
//! Upstream: doctrine/collections tests/AbstractLazyArrayCollectionTest.php

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

use common::array_collection_test_case;
use common::lazy_array_collection;
use php_value::{PhpArray, php_array};
use php2rust_doctrine_collections::{Collection, ReadableCollection};

/// `buildCollection($elements)`.
fn build_collection(elements: PhpArray) -> Box<dyn Collection> {
    Box::new(lazy_array_collection(elements))
}

// The tests of `ArrayCollectionTestCase`.

#[test]
fn testToArray() {
    array_collection_test_case::testToArray(build_collection);
}

#[test]
fn testFirst() {
    array_collection_test_case::testFirst(build_collection);
}

#[test]
fn testLast() {
    array_collection_test_case::testLast(build_collection);
}

#[test]
fn testKey() {
    array_collection_test_case::testKey(build_collection);
}

#[test]
fn testNext() {
    array_collection_test_case::testNext(build_collection);
}

#[test]
fn testCurrent() {
    array_collection_test_case::testCurrent(build_collection);
}

#[test]
fn testGetKeys() {
    array_collection_test_case::testGetKeys(build_collection);
}

#[test]
fn testGetValues() {
    array_collection_test_case::testGetValues(build_collection);
}

#[test]
fn testCount() {
    array_collection_test_case::testCount(build_collection);
}

#[test]
fn testIterator() {
    array_collection_test_case::testIterator(build_collection);
}

#[test]
fn testRemove() {
    array_collection_test_case::testRemove(build_collection);
}

#[test]
fn testRemoveElement() {
    array_collection_test_case::testRemoveElement(build_collection);
}

#[test]
fn testContainsKey() {
    array_collection_test_case::testContainsKey(build_collection);
}

#[test]
fn testEmpty() {
    array_collection_test_case::testEmpty(build_collection);
}

#[test]
fn testContains() {
    array_collection_test_case::testContains(build_collection);
}

#[test]
fn testExists() {
    array_collection_test_case::testExists(build_collection);
}

#[test]
fn testFindFirst() {
    array_collection_test_case::testFindFirst(build_collection);
}

#[test]
fn testFindFirstNotFound() {
    array_collection_test_case::testFindFirstNotFound(build_collection);
}

#[test]
fn testIndexOf() {
    array_collection_test_case::testIndexOf(build_collection);
}

#[test]
fn testGet() {
    array_collection_test_case::testGet(build_collection);
}

#[test]
fn testMatchingWithSortingPreserveKeys() {
    array_collection_test_case::testMatchingWithSortingPreserveKeys(build_collection);
}

#[test]
fn testLegacyMatchingWithSortingPreserveKeys() {
    array_collection_test_case::testLegacyMatchingWithSortingPreserveKeys(build_collection);
}

#[test]
fn testMatchingWithSlicingPreserveKeys() {
    array_collection_test_case::testMatchingWithSlicingPreserveKeys(build_collection);
}

#[test]
fn testMultiColumnSortAppliesAllSorts() {
    array_collection_test_case::testMultiColumnSortAppliesAllSorts(build_collection);
}

#[test]
fn testLazyCollection() {
    let mut collection = lazy_array_collection(php_array!["a", "b", "c"]);

    assert!(!collection.is_initialized());
    assert_eq!(3, collection.count().unwrap());
}
