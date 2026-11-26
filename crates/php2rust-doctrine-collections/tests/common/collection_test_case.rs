//! `CollectionTestCase`: the tests CollectionTest and
//! AbstractLazyCollectionTest share, as functions of the test's `setUp()`
//! (the `$this->collection` it builds); each test file runs them under
//! their names.
//!
//! Upstream: doctrine/collections tests/CollectionTestCase.php

#![allow(non_snake_case)]

use php_value::{Value, php_array};
use php2rust_doctrine_collections::{Collection, Criteria};

use super::{TestObject, assert_equals, k};

/// `is_numeric($value)`.
pub fn is_numeric(value: &Value) -> bool {
    match value {
        Value::Int(_) | Value::Float(_) => true,
        Value::String(s) => php_value::numeric::numeric_value(s.as_bytes()).is_some(),
        _ => false,
    }
}

/// `setUp()`: the `$this->collection` it builds.
pub type SetUp = fn() -> Box<dyn Collection>;

pub fn testIssetAndUnset(set_up: SetUp) {
    let mut collection = set_up();
    assert!(!collection.offset_exists(&k(0)).unwrap());
    collection.add(Value::from("testing")).unwrap();
    assert!(collection.offset_exists(&k(0)).unwrap());
    collection.offset_unset(&k(0)).unwrap();
    assert!(!collection.offset_exists(&k(0)).unwrap());
}

pub fn testRemovingNonExistentEntryReturnsNull(set_up: SetUp) {
    let mut collection = set_up();
    assert_equals(
        &Value::Null,
        &collection.remove(&k("testing_does_not_exist")).unwrap(),
    );
}

pub fn testExists(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::from("one")).unwrap();
    collection.add(Value::from("two")).unwrap();
    let exists = collection
        .exists(&mut |_, e| *e == Value::from("one"))
        .unwrap();
    assert!(exists);
    let exists = collection
        .exists(&mut |_, e| *e == Value::from("other"))
        .unwrap();
    assert!(!exists);
}

pub fn testFindFirst(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::from("one")).unwrap();
    collection.add(Value::from("two")).unwrap();
    let one = collection
        .find_first(&mut |_, e| *e == Value::from("one"))
        .unwrap();
    assert_eq!(Value::from("one"), one);
}

pub fn testFindFirstNotFound(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::from("one")).unwrap();
    collection.add(Value::from("two")).unwrap();
    let other = collection
        .find_first(&mut |_, e| *e == Value::from("other"))
        .unwrap();
    assert_eq!(Value::Null, other);
}

pub fn testMap(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::Int(1)).unwrap();
    collection.add(Value::Int(2)).unwrap();
    let mut res = collection
        .map(&mut |e| php_value::ops::mul(e, &Value::Int(2)).unwrap())
        .unwrap();
    assert_equals(
        &Value::Array(php_array![2, 4]),
        &Value::Array(res.to_array().unwrap()),
    );
}

pub fn testReduce(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::Int(1)).unwrap();
    collection.add(Value::Int(2)).unwrap();
    collection.add(Value::Int(3)).unwrap();
    collection.add(Value::Int(4)).unwrap();

    let res = collection
        .reduce(
            &mut |sum, e| php_value::ops::add(&sum, e).unwrap(),
            Value::Null,
        )
        .unwrap();
    assert_eq!(Value::Int(10), res);
}

pub fn testFilter(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::Int(1)).unwrap();
    collection.add(Value::from("foo")).unwrap();
    collection.add(Value::Int(3)).unwrap();
    let mut res = collection.filter(&mut |e, _| is_numeric(e)).unwrap();
    assert_equals(
        &Value::Array(php_array![0 => 1, 2 => 3]),
        &Value::Array(res.to_array().unwrap()),
    );
}

pub fn testFilterByValueAndKey(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::Int(1)).unwrap();
    collection.add(Value::from("foo")).unwrap();
    collection.add(Value::Int(3)).unwrap();
    collection.add(Value::Int(4)).unwrap();
    collection.add(Value::Int(5)).unwrap();
    let mut res = collection
        .filter(&mut |v, k| is_numeric(v) && k.as_int().is_some_and(|k| k % 2 == 0))
        .unwrap();
    assert_eq!(php_array![0 => 1, 2 => 3, 4 => 5], res.to_array().unwrap());
}

pub fn testFirstAndLast(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::from("one")).unwrap();
    collection.add(Value::from("two")).unwrap();

    assert_equals(&collection.first().unwrap(), &Value::from("one"));
    assert_equals(&collection.last().unwrap(), &Value::from("two"));
}

pub fn testArrayAccess(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();

    assert_equals(&collection.offset_get(&k(0)).unwrap(), &Value::from("one"));
    assert_equals(&collection.offset_get(&k(1)).unwrap(), &Value::from("two"));

    collection.offset_unset(&k(0)).unwrap();
    assert_eq!(collection.count().unwrap(), 1);
}

pub fn testContainsKey(set_up: SetUp) {
    let mut collection = set_up();
    collection
        .offset_set(Some(k(5)), Value::from("five"))
        .unwrap();
    assert!(collection.contains_key(&k(5)).unwrap());
}

pub fn testContains(set_up: SetUp) {
    let mut collection = set_up();
    collection
        .offset_set(Some(k(0)), Value::from("test"))
        .unwrap();
    assert!(collection.contains(&Value::from("test")).unwrap());
}

pub fn testSearch(set_up: SetUp) {
    let mut collection = set_up();
    collection
        .offset_set(Some(k(0)), Value::from("test"))
        .unwrap();
    assert_equals(
        &Value::Int(0),
        &collection
            .index_of(&Value::from("test"))
            .unwrap()
            .map_or(Value::Bool(false), |key| key.to_value()),
    );
}

pub fn testGet(set_up: SetUp) {
    let mut collection = set_up();
    collection
        .offset_set(Some(k(0)), Value::from("test"))
        .unwrap();
    assert_equals(&Value::from("test"), &collection.get(&k(0)).unwrap());
}

pub fn testGetKeys(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
    assert_equals(
        &Value::Array(php_array![0, 1]),
        &Value::Array(collection.get_keys().unwrap()),
    );
}

pub fn testGetValues(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
    assert_equals(
        &Value::Array(php_array!["one", "two"]),
        &Value::Array(collection.get_values().unwrap()),
    );
}

pub fn testCount(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
    assert_eq!(collection.count().unwrap(), 2);
    // count($this->collection) is Countable::count().
    assert_eq!(collection.count().unwrap(), 2);
}

pub fn testForAll(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
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

pub fn testPartition(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::Bool(true)).unwrap();
    collection.offset_set(None, Value::Bool(false)).unwrap();
    let (mut matches, mut no_matches) = collection
        .partition(&mut |_, e| *e == Value::Bool(true))
        .unwrap();
    assert_equals(&matches.offset_get(&k(0)).unwrap(), &Value::Bool(true));
    assert_equals(&no_matches.offset_get(&k(0)).unwrap(), &Value::Bool(false));
}

pub fn testClear(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
    collection.clear().unwrap();
    assert!(collection.is_empty().unwrap());
}

pub fn testRemove(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
    let el = collection.remove(&k(0)).unwrap();

    assert_equals(&Value::from("one"), &el);
    assert!(!collection.contains(&Value::from("one")).unwrap());
    assert_eq!(Value::Null, collection.remove(&k(0)).unwrap());
}

pub fn testRemoveElement(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();

    assert!(collection.remove_element(&Value::from("two")).unwrap());
    assert!(!collection.contains(&Value::from("two")).unwrap());
    assert!(!collection.remove_element(&Value::from("two")).unwrap());
}

pub fn testSlice(set_up: SetUp) {
    let mut collection = set_up();
    collection.offset_set(None, Value::from("one")).unwrap();
    collection.offset_set(None, Value::from("two")).unwrap();
    collection.offset_set(None, Value::from("three")).unwrap();

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

/// `fillMatchingFixture()`.
pub fn fill_matching_fixture(collection: &mut dyn Collection) {
    let obj1 = TestObject::new(Value::Null, Value::Null);
    obj1.set_foo("bar");
    collection.offset_set(None, Value::Object(obj1)).unwrap();

    let obj2 = TestObject::new(Value::Null, Value::Null);
    obj2.set_foo("baz");
    collection.offset_set(None, Value::Object(obj2)).unwrap();
}

pub fn testCanRemoveNullValuesByKey(set_up: SetUp) {
    let mut collection = set_up();
    collection.add(Value::Null).unwrap();
    collection.remove(&k(0)).unwrap();
    assert!(collection.is_empty().unwrap());
}

pub fn testCanVerifyExistingKeysWithNullValues(set_up: SetUp) {
    let mut collection = set_up();
    collection.set(k("key"), Value::Null).unwrap();
    assert!(collection.contains_key(&k("key")).unwrap());
}

pub fn testMatchingAlwaysReturnsCollection(set_up: SetUp) {
    let mut collection = set_up();
    let criteria = Criteria::create();

    // A `Box<dyn Collection>`, which is `Selectable` too.
    let mut matched: Box<dyn Collection> = collection.matching(&criteria).unwrap();
    let _: Box<dyn Collection> = matched.matching(&criteria).unwrap();
}
