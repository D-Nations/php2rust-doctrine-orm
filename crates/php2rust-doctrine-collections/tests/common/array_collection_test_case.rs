//! `ArrayCollectionTestCase`: the tests ArrayCollectionTest and
//! AbstractLazyArrayCollectionTest share, as functions of the test's
//! `buildCollection()`; each test file runs them under their names. The
//! data providers are written in place.
//!
//! Upstream: doctrine/collections tests/ArrayCollectionTestCase.php

#![allow(non_snake_case)]

use php_value::{Key, PhpArray, Value, php_array};
use php2rust_doctrine_collections::{Collection, Criteria, Order};

use super::{TestObject, TestObjectPrivatePropertyOnly, assert_equals, k};

/// `provideDifferentElements()`.
pub fn provide_different_elements() -> Vec<(&'static str, PhpArray)> {
    let mut mixed = PhpArray::new();
    mixed.insert("A", Value::from("a"));
    mixed.push(Value::Int(1)).unwrap();
    mixed.insert("B", Value::from("b"));
    mixed.push(Value::Int(2)).unwrap();
    mixed.push(Value::Int(3)).unwrap();
    vec![
        ("indexed", php_array![1, 2, 3, 4, 5]),
        (
            "associative",
            php_array!["A" => "a", "B" => "b", "C" => "c"],
        ),
        ("mixed", mixed),
    ]
}

/// `provideSlices()`: elements, sliced elements, first result, max result.
pub fn provide_slices() -> Vec<(&'static str, PhpArray, PhpArray, i64, Option<i64>)> {
    let abcd = php_array!["a" => 1, "b" => 2, "c" => 3, "d" => 4];
    vec![
        (
            "preserve numeric keys",
            php_array![0 => 1, 1 => 2, 2 => 3, 3 => 4],
            php_array![1 => 2, 2 => 3],
            1,
            Some(2),
        ),
        (
            "preserve string keys",
            abcd.clone(),
            php_array!["b" => 2, "c" => 3],
            1,
            Some(2),
        ),
        (
            "preserve keys on firstresult only",
            abcd.clone(),
            php_array!["b" => 2, "c" => 3, "d" => 4],
            1,
            None,
        ),
        (
            "preserve keys on maxresult only",
            abcd,
            php_array!["a" => 1, "b" => 2],
            0,
            Some(2),
        ),
    ]
}

/// A PHP array with its internal pointer, for `reset()`, `end()`,
/// `key()`, `next()` and `current()` on the test's own array.
pub struct ArrayPointer {
    pairs: Vec<(Key, Value)>,
    position: usize,
}

impl ArrayPointer {
    pub fn new(array: &PhpArray) -> Self {
        Self {
            pairs: array.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            position: 0,
        }
    }

    pub fn reset(&mut self) -> Value {
        self.position = 0;
        self.current()
    }

    pub fn end(&mut self) -> Value {
        self.position = self.pairs.len().saturating_sub(1);
        self.current()
    }

    pub fn key(&self) -> Option<Key> {
        self.pairs.get(self.position).map(|(k, _)| k.clone())
    }

    pub fn current(&self) -> Value {
        self.pairs
            .get(self.position)
            .map_or(Value::Bool(false), |(_, v)| v.clone())
    }

    pub fn next(&mut self) -> Value {
        if self.position < self.pairs.len() {
            self.position += 1;
        }
        self.current()
    }
}

/// `buildCollection($elements)`.
pub type Build = fn(PhpArray) -> Box<dyn Collection>;

pub fn testToArray(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        assert_eq!(elements, collection.to_array().unwrap());
    }
}

pub fn testFirst(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        assert_eq!(
            ArrayPointer::new(&elements).reset(),
            collection.first().unwrap()
        );
    }
}

pub fn testLast(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        assert_eq!(
            ArrayPointer::new(&elements).end(),
            collection.last().unwrap()
        );
    }
}

pub fn testKey(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        let mut array = ArrayPointer::new(&elements);

        assert_eq!(array.key(), collection.key().unwrap());

        array.next();
        collection.next().unwrap();

        assert_eq!(array.key(), collection.key().unwrap());
    }
}

pub fn testNext(build: Build) {
    for (_, elements) in provide_different_elements() {
        let count = elements.len();
        let mut collection = build(elements.clone());
        let mut array = ArrayPointer::new(&elements);

        for _ in 0..count {
            let collection_next = collection.next().unwrap();
            let array_next = array.next();

            if !collection_next.is_truthy() || !array_next.is_truthy() {
                break;
            }

            assert_eq!(
                array_next, collection_next,
                "Returned value of ArrayCollection::next() and next() not match"
            );
            assert_eq!(array.key(), collection.key().unwrap(), "Keys not match");
            assert_eq!(
                array.current(),
                collection.current().unwrap(),
                "Current values not match"
            );
        }

        assert_eq!(Value::Bool(false), collection.next().unwrap());
    }
}

pub fn testCurrent(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        let mut array = ArrayPointer::new(&elements);

        assert_eq!(array.current(), collection.current().unwrap());

        array.next();
        collection.next().unwrap();

        assert_eq!(array.current(), collection.current().unwrap());
    }
}

pub fn testGetKeys(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        let keys = php_value::PhpArray::list(elements.keys().map(php_value::Key::to_value));
        assert_eq!(keys, collection.get_keys().unwrap());
    }
}

pub fn testGetValues(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        let values = php_value::PhpArray::list(elements.values().cloned());
        assert_eq!(values, collection.get_values().unwrap());
    }
}

pub fn testCount(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());
        assert_eq!(elements.len(), collection.count().unwrap());
    }
}

pub fn testIterator(build: Build) {
    for (_, elements) in provide_different_elements() {
        let mut collection = build(elements.clone());

        let mut iterations = 0;
        for (key, item) in collection.get_iterator().unwrap().iter() {
            assert_eq!(elements.get(key), Some(item), "Item {key:?} not match");
            iterations += 1;
        }

        assert_eq!(elements.len(), iterations, "Number of iterations not match");
    }
}

pub fn testRemove(build: Build) {
    let mut elements = php_value::PhpArray::new();
    elements.push(Value::Int(1)).unwrap();
    elements.insert("A", Value::from("a"));
    elements.push(Value::Int(2)).unwrap();
    elements.insert("B", Value::from("b"));
    elements.push(Value::Int(3)).unwrap();
    let mut collection = build(elements.clone());

    assert_equals(&Value::Int(1), &collection.remove(&k(0)).unwrap());
    elements.remove(&k(0));

    assert_equals(
        &Value::Null,
        &collection.remove(&k("non-existent")).unwrap(),
    );
    elements.remove(&k("non-existent"));

    assert_equals(&Value::Int(2), &collection.remove(&k(1)).unwrap());
    elements.remove(&k(1));

    assert_equals(&Value::from("a"), &collection.remove(&k("A")).unwrap());
    elements.remove(&k("A"));

    assert_equals(
        &Value::Array(elements),
        &Value::Array(collection.to_array().unwrap()),
    );
}

pub fn testRemoveElement(build: Build) {
    let mut elements = php_value::PhpArray::new();
    elements.push(Value::Int(1)).unwrap();
    elements.insert("A", Value::from("a"));
    elements.push(Value::Int(2)).unwrap();
    elements.insert("B", Value::from("b"));
    elements.push(Value::Int(3)).unwrap();
    elements.insert("A2", Value::from("a"));
    elements.insert("B2", Value::from("b"));
    let mut collection = build(elements.clone());

    assert!(collection.remove_element(&Value::Int(1)).unwrap());
    elements.remove(&k(0));

    assert!(
        !collection
            .remove_element(&Value::from("non-existent"))
            .unwrap()
    );

    assert!(collection.remove_element(&Value::from("a")).unwrap());
    elements.remove(&k("A"));

    assert!(collection.remove_element(&Value::from("a")).unwrap());
    elements.remove(&k("A2"));

    assert_equals(
        &Value::Array(elements),
        &Value::Array(collection.to_array().unwrap()),
    );
}

/// `[1, 'A' => 'a', 2, 'null' => null, 3, 'A2' => 'a', <last>]`.
fn mixed_elements(last_key: &str, last: Value) -> php_value::PhpArray {
    let mut elements = php_value::PhpArray::new();
    elements.push(Value::Int(1)).unwrap();
    elements.insert("A", Value::from("a"));
    elements.push(Value::Int(2)).unwrap();
    elements.insert("null", Value::Null);
    elements.push(Value::Int(3)).unwrap();
    elements.insert("A2", Value::from("a"));
    elements.insert(last_key, last);
    elements
}

pub fn testContainsKey(build: Build) {
    let elements = mixed_elements("B2", Value::from("b"));
    let mut collection = build(elements);

    assert!(collection.contains_key(&k(0)).unwrap(), "Contains index 0");
    assert!(
        collection.contains_key(&k("A")).unwrap(),
        "Contains key \"A\""
    );
    assert!(
        collection.contains_key(&k("null")).unwrap(),
        "Contains key \"null\", with value null"
    );
    assert!(
        !collection.contains_key(&k("non-existent")).unwrap(),
        "Doesn't contain key"
    );
}

pub fn testEmpty(build: Build) {
    let mut collection = build(php_value::PhpArray::new());
    assert!(collection.is_empty().unwrap(), "Empty collection");

    collection.add(Value::Int(1)).unwrap();
    assert!(!collection.is_empty().unwrap(), "Not empty collection");
}

pub fn testContains(build: Build) {
    let elements = mixed_elements("zero", Value::Int(0));
    let mut collection = build(elements);

    assert!(
        collection.contains(&Value::Int(0)).unwrap(),
        "Contains Zero"
    );
    assert!(
        collection.contains(&Value::from("a")).unwrap(),
        "Contains \"a\""
    );
    assert!(collection.contains(&Value::Null).unwrap(), "Contains Null");
    assert!(
        !collection.contains(&Value::from("non-existent")).unwrap(),
        "Doesn't contain an element"
    );
}

pub fn testExists(build: Build) {
    let elements = mixed_elements("zero", Value::Int(0));
    let mut collection = build(elements);

    assert!(
        collection
            .exists(&mut |key, element| *key == k("A") && *element == Value::from("a"))
            .unwrap(),
        "Element exists"
    );

    assert!(
        !collection
            .exists(&mut |key, element| *key == k("non-existent")
                && *element == Value::from("non-existent"))
            .unwrap(),
        "Element not exists"
    );
}

pub fn testFindFirst(build: Build) {
    let elements = mixed_elements("zero", Value::Int(0));
    let mut collection = build(elements);

    assert_eq!(
        Value::from("a"),
        collection
            .find_first(&mut |key, element| *key == k("A") && *element == Value::from("a"))
            .unwrap(),
        "Element exists"
    );
}

pub fn testFindFirstNotFound(build: Build) {
    let elements = mixed_elements("zero", Value::Int(0));
    let mut collection = build(elements);

    assert_eq!(
        Value::Null,
        collection
            .find_first(&mut |key, element| *key == k("non-existent")
                && *element == Value::from("non-existent"))
            .unwrap(),
        "Element does not exists"
    );
}

pub fn testIndexOf(build: Build) {
    let elements = mixed_elements("zero", Value::Int(0));
    let mut collection = build(elements.clone());

    let search = |needle: &Value| {
        elements
            .iter()
            .find(|(_, v)| *v == needle)
            .map(|(key, _)| key.clone())
    };
    assert_eq!(
        search(&Value::Int(2)),
        collection.index_of(&Value::Int(2)).unwrap(),
        "Index of 2"
    );
    assert_eq!(
        search(&Value::Null),
        collection.index_of(&Value::Null).unwrap(),
        "Index of null"
    );
    assert_eq!(
        search(&Value::from("non-existent")),
        collection.index_of(&Value::from("non-existent")).unwrap(),
        "Index of non existent"
    );
}

pub fn testGet(build: Build) {
    let elements = mixed_elements("zero", Value::Int(0));
    let mut collection = build(elements);

    assert_eq!(
        Value::Int(2),
        collection.get(&k(1)).unwrap(),
        "Get element by index"
    );
    assert_eq!(
        Value::from("a"),
        collection.get(&k("A")).unwrap(),
        "Get element by name"
    );
    assert_eq!(
        Value::Null,
        collection.get(&k("non-existent")).unwrap(),
        "Get non existent element"
    );
}

pub fn testMatchingWithSortingPreserveKeys(build: Build) {
    let object1 = TestObjectPrivatePropertyOnly::object(2);
    let object2 = TestObjectPrivatePropertyOnly::object(1);

    let mut collection =
        build(php_array!["object1" => object1.clone(), "object2" => object2.clone()]);

    assert_eq!(
        php_array!["object2" => object2, "object1" => object1],
        collection
            .matching(&Criteria::new(
                None,
                Some(vec![("fooBar".to_owned(), Order::Ascending.into())]),
                0,
                None,
            ))
            .unwrap()
            .to_array()
            .unwrap(),
    );
}

pub fn testLegacyMatchingWithSortingPreserveKeys(build: Build) {
    let object1 = TestObject::new(Value::Null, Value::Null);
    let object2 = TestObject::new(Value::Null, Value::Null);

    object1.set_foo(2);
    object2.set_foo(1);

    let (object1, object2) = (Value::Object(object1), Value::Object(object2));
    let mut collection =
        build(php_array!["object1" => object1.clone(), "object2" => object2.clone()]);

    assert_eq!(
        php_array!["object2" => object2, "object1" => object1],
        collection
            .matching(&Criteria::new(
                None,
                Some(vec![("foo".to_owned(), Order::Ascending.into())]),
                0,
                None,
            ))
            .unwrap()
            .to_array()
            .unwrap(),
    );
}

pub fn testMatchingWithSlicingPreserveKeys(build: Build) {
    for (_, array, sliced_array, first_result, max_result) in provide_slices() {
        let mut collection = build(array);

        assert_eq!(
            sliced_array,
            collection
                .matching(&Criteria::new(None, None, first_result, max_result))
                .unwrap()
                .to_array()
                .unwrap(),
        );
    }
}

pub fn testMultiColumnSortAppliesAllSorts(build: Build) {
    let mut collection = build(php_array![
        php_array!["foo" => 1, "bar" => 2],
        php_array!["foo" => 2, "bar" => 4],
        php_array!["foo" => 2, "bar" => 3],
    ]);

    let expected = php_array![
        1 => php_array!["foo" => 2, "bar" => 4],
        2 => php_array!["foo" => 2, "bar" => 3],
        0 => php_array!["foo" => 1, "bar" => 2],
    ];

    assert_eq!(
        expected,
        collection
            .matching(&Criteria::new(
                None,
                Some(vec![
                    ("foo".to_owned(), Order::Descending.into()),
                    ("bar".to_owned(), Order::Descending.into()),
                ]),
                0,
                None,
            ))
            .unwrap()
            .to_array()
            .unwrap(),
    );
}
