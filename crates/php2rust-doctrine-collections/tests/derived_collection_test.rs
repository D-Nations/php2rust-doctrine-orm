//! Port of tests/DerivedCollectionTest.php (and its
//! DerivedArrayCollection.php fixture) at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d. The `assertInstanceOf`
//! checks are the result types.
//!
//! Upstream: doctrine/collections tests/DerivedCollectionTest.php tests/DerivedArrayCollection.php

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

use php_value::object::StdClass;
use php_value::{PhpArray, Value};
use php2rust_doctrine_collections::{ArrayCollection, ArrayCollectionClass, Criteria};

/// `DerivedArrayCollection`: a constructor that takes a `stdClass`
/// first, which `createFrom()` passes on.
#[derive(Debug, Clone)]
struct Derived {
    foo: Value,
}

impl ArrayCollectionClass for Derived {}

type DerivedArrayCollection = ArrayCollection<Derived>;

#[test]
fn testDerivedClassCreation() {
    let collection = DerivedArrayCollection::with_class(
        Derived {
            foo: StdClass::object(PhpArray::new()),
        },
        PhpArray::new(),
    );
    let closure = |_: &Value| Value::Bool(false);

    let mapped: DerivedArrayCollection = collection.map(closure);
    let filtered: DerivedArrayCollection = collection.filter(|v, _| closure(v).is_truthy());
    let (matches, no_matches): (DerivedArrayCollection, DerivedArrayCollection) =
        collection.partition(|_, v| closure(v).is_truthy());
    let selected: DerivedArrayCollection = collection.matching(&Criteria::create()).unwrap();

    for derived in [&mapped, &filtered, &matches, &no_matches, &selected] {
        assert!(derived.class().foo.identical(&collection.class().foo));
    }
}
