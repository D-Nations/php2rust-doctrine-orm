//! LazyCriteriaCollectionTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! PHPUnit's mock of `EntityPersister` is an `EntityPersister` that returns
//! the configured results and counts its `count()` and `loadCriteria()`
//! calls; `->with($this->criteria)` is checked by comparing the criteria.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/LazyCriteriaCollectionTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use php_value::{Object, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object::{self as coll, CollectionRef};
use php2rust_doctrine_orm::lazy_criteria_collection::LazyCriteriaCollection;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::persisters::EntityPersister;
use php2rust_doctrine_orm::unit_of_work::Fut;

use orm::s;

const TEST_OBJECT: &str = "Doctrine\\Tests\\ORM\\LazyCriteriaCollectionTestObject";

#[derive(Debug)]
struct PersisterMock {
    criteria: Criteria,
    count: i64,
    loaded: Vec<Value>,
    count_calls: AtomicUsize,
    load_criteria_calls: AtomicUsize,
}

impl PersisterMock {
    fn new(criteria: &Criteria, count: i64, loaded: Vec<Value>) -> Arc<Self> {
        Arc::new(Self {
            criteria: criteria.clone(),
            count,
            loaded,
            count_calls: AtomicUsize::new(0),
            load_criteria_calls: AtomicUsize::new(0),
        })
    }

    fn count_calls(&self) -> usize {
        self.count_calls.load(Ordering::SeqCst)
    }

    fn load_criteria_calls(&self) -> usize {
        self.load_criteria_calls.load(Ordering::SeqCst)
    }
}

impl EntityPersister for PersisterMock {
    fn count_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, i64> {
        assert_eq!(*criteria, self.criteria);
        self.count_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(self.count) })
    }

    fn load_criteria<'a>(&'a self, criteria: &'a Criteria) -> Fut<'a, Vec<Value>> {
        assert_eq!(*criteria, self.criteria);
        self.load_criteria_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move { Ok(self.loaded.clone()) })
    }

    fn exists<'a>(&'a self, _entity: &'a Object, _extra: Option<&'a Criteria>) -> Fut<'a, bool> {
        Box::pin(async { Ok(false) })
    }

    fn get_select_condition_statement_sql(
        &self,
        _field: &str,
        _value: &Value,
        _assoc: Option<&AssociationMapping>,
        _comparison: Option<&str>,
    ) -> Result<String, Thrown> {
        Ok(String::new())
    }
}

fn set_up(count: i64, loaded: Vec<Value>) -> (Arc<PersisterMock>, Arc<LazyCriteriaCollection>) {
    let criteria = Criteria::create();
    let persister = PersisterMock::new(&criteria, count, loaded);
    let lazy_criteria_collection = LazyCriteriaCollection::new(persister.clone(), criteria);
    (persister, lazy_criteria_collection)
}

fn foo_bar_baz() -> Vec<Value> {
    vec![s("foo"), s("bar"), s("baz")]
}

#[tokio::test]
async fn testCountIsCached() {
    let (persister, lazy_criteria_collection) = set_up(10, Vec::new());

    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 10);
    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 10);
    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 10);
    assert_eq!(persister.count_calls(), 1);
}

#[tokio::test]
async fn testCountIsCachedEvenWithZeroResult() {
    let (persister, lazy_criteria_collection) = set_up(0, Vec::new());

    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 0);
    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 0);
    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 0);
    assert_eq!(persister.count_calls(), 1);
}

#[tokio::test]
async fn testCountUsesWrappedCollectionWhenInitialized() {
    let (persister, lazy_criteria_collection) = set_up(0, foo_bar_baz());

    assert!(
        Value::Array(lazy_criteria_collection.to_array().await.unwrap())
            .identical(&Value::Array(PhpArray::list(foo_bar_baz())))
    );

    assert_eq!(lazy_criteria_collection.count().await.unwrap(), 3);
    assert_eq!(persister.load_criteria_calls(), 1);
    // should never call the persister's count
    assert_eq!(persister.count_calls(), 0);
}

#[tokio::test]
async fn testMatchingUsesThePersisterOnlyOnce() {
    let foo = orm::new_object(TEST_OBJECT);
    let bar = orm::new_object(TEST_OBJECT);
    let baz = orm::new_object(TEST_OBJECT);

    orm::set(&foo, "val", s("foo"));
    orm::set(&bar, "val", s("bar"));
    orm::set(&baz, "val", s("baz"));

    let (persister, lazy_criteria_collection) = set_up(
        0,
        vec![
            Value::Object(foo.clone()),
            Value::Object(bar),
            Value::Object(baz),
        ],
    );

    let mut criteria = Criteria::create();
    criteria
        .and_where(Criteria::expr().eq("val", s("foo")))
        .unwrap();

    let filtered = lazy_criteria_collection.matching(&criteria).await.unwrap();

    assert!(matches!(
        CollectionRef::of(&filtered),
        Some(CollectionRef::Array(_))
    ));
    let elements = coll::to_array(&filtered).await.unwrap();
    assert_eq!(elements.len(), 1);
    assert!(orm::same_value(elements.values().next().unwrap(), &foo));

    let again = lazy_criteria_collection.matching(&criteria).await.unwrap();
    let elements = coll::to_array(&again).await.unwrap();
    assert_eq!(elements.len(), 1);
    assert!(orm::same_value(elements.values().next().unwrap(), &foo));
    assert_eq!(persister.load_criteria_calls(), 1);
}

#[tokio::test]
async fn testIsEmptyUsesCountWhenNotInitialized() {
    let (persister, lazy_criteria_collection) = set_up(0, Vec::new());

    assert!(lazy_criteria_collection.is_empty().await.unwrap());
    assert_eq!(persister.count_calls(), 1);
}

#[tokio::test]
async fn testIsEmptyIsFalseIfCountIsNotZero() {
    let (persister, lazy_criteria_collection) = set_up(1, Vec::new());

    assert!(!lazy_criteria_collection.is_empty().await.unwrap());
    assert_eq!(persister.count_calls(), 1);
}

#[tokio::test]
async fn testIsEmptyUsesWrappedCollectionWhenInitialized() {
    let (persister, lazy_criteria_collection) = set_up(0, foo_bar_baz());

    assert!(
        Value::Array(lazy_criteria_collection.to_array().await.unwrap())
            .identical(&Value::Array(PhpArray::list(foo_bar_baz())))
    );

    assert!(!lazy_criteria_collection.is_empty().await.unwrap());
    assert_eq!(persister.load_criteria_calls(), 1);
    // should never call the persister's count
    assert_eq!(persister.count_calls(), 0);
}
