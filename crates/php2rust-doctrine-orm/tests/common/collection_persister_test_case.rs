//! `CollectionPersisterTestCase`'s tests, for each cached collection
//! persister: each test file calls them with a `Fixture` over its
//! `create_persister()` and `create_region()`.
//!
//! `createMock()` expectations are checked after the call: a mock
//! records its calls. `assertInstanceOf()` of an interface is the type;
//! `identicalTo()` of a collection compares the `Arc`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Cache/Persister/Collection/CollectionPersisterTestCase.php

#![allow(dead_code, reason = "each test file uses a subset")]
#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::{Key, Object, PhpArray, Value};
use php2rust_doctrine_collections::ArrayCollection;
use php2rust_doctrine_orm::cache::{
    AbstractCollectionPersister, CachedCollectionPersister, CachedPersister, Region,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::AssociationMapping;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::persisters::collection::CollectionPersister;

use crate::slc::{self, Arg, CollectionPersisterMock, RegionMock, Ret, Slc};

/// `createPersister($em, $persister, $region, $mapping)`.
pub type CreatePersister = fn(
    &Arc<EntityManager>,
    Arc<dyn CollectionPersister>,
    Arc<dyn Region>,
    AssociationMapping,
) -> AbstractCollectionPersister;

/// The test's entity manager, region mock and collection persister mock.
pub struct Fixture {
    pub slc: Slc,
    pub region: Arc<RegionMock>,
    pub collection_persister: Arc<CollectionPersisterMock>,
    create: CreatePersister,
}

impl Fixture {
    /// `setUp()` with `createRegion()`.
    pub fn new(create: CreatePersister, region: Arc<RegionMock>) -> Self {
        Self {
            slc: slc::slc_entity_manager(),
            region,
            collection_persister: CollectionPersisterMock::new(),
            create,
        }
    }

    pub fn em(&self) -> &Arc<EntityManager> {
        &self.slc.em
    }

    /// `$class->associationMappings['cities']` of `State`.
    pub fn cities(&self) -> AssociationMapping {
        self.em()
            .get_class_metadata(slc::STATE)
            .unwrap()
            .read()
            .association_mappings
            .get("cities")
            .cloned()
            .unwrap()
    }

    /// `createCollection($owner)`.
    pub fn create_collection(&self, owner: &Object) -> Arc<PersistentCollection> {
        let class = self.em().get_class_metadata(slc::STATE).unwrap();
        let assoc = self.cities();
        let coll = PersistentCollection::new(
            Some(self.em()),
            Some(class),
            ArrayCollection::new(PhpArray::new()),
        );

        coll.set_owner(owner, &assoc);
        coll.set_initialized(true);

        coll
    }

    /// `createPersisterDefault()`.
    pub fn create_persister_default(&self) -> AbstractCollectionPersister {
        (self.create)(
            self.em(),
            self.collection_persister.clone(),
            self.region.clone(),
            self.cities(),
        )
    }

    /// `registerManaged($entity, ['id' => 1], ['id' => 1, 'name' => 'Foo'])`.
    pub fn register_managed(&self, entity: &Object) {
        self.em()
            .get_unit_of_work()
            .register_managed(
                entity,
                slc::id(1),
                slc::array(&[("id", Value::Int(1)), ("name", Value::string("Foo"))]),
            )
            .unwrap();
    }
}

/// `(object) []`.
fn std_object() -> Object {
    Arc::new(php_value::object::StdClass::default())
}

fn implements_collection_persister<T: CollectionPersister + ?Sized>(_: &T) {}
fn implements_cached_persister<T: CachedPersister + ?Sized>(_: &T) {}
fn implements_cached_collection_persister<T: CachedCollectionPersister + ?Sized>(_: &T) {}

pub fn testImplementsEntityPersister(f: &Fixture) {
    let persister = f.create_persister_default();

    implements_collection_persister(&persister);
    implements_cached_persister(&persister);
    implements_cached_collection_persister(&persister);
}

pub async fn testInvokeDelete(f: &Fixture) {
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);

    f.register_managed(&entity);

    CollectionPersister::delete(&persister, &collection)
        .await
        .unwrap();

    f.collection_persister.recorder.assert_once_with(
        "delete",
        &[CollectionPersisterMock::collection(&collection)],
    );
}

pub async fn testInvokeUpdate(f: &Fixture) {
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);

    collection.set_dirty(true);

    f.register_managed(&entity);

    CollectionPersister::update(&persister, &collection)
        .await
        .unwrap();

    f.collection_persister.recorder.assert_once_with(
        "update",
        &[CollectionPersisterMock::collection(&collection)],
    );
}

pub async fn testInvokeCount(f: &Fixture) {
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);

    f.register_managed(&entity);

    f.collection_persister
        .recorder
        .will_return("count", Ret::Int(0));

    assert_eq!(0, persister.count(&collection).await.unwrap());
    f.collection_persister
        .recorder
        .assert_once_with("count", &[CollectionPersisterMock::collection(&collection)]);
}

pub async fn testInvokeSlice(f: &Fixture) {
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);
    let slice = PhpArray::list([Value::Object(std_object()), Value::Object(std_object())]);

    f.register_managed(&entity);

    f.collection_persister
        .recorder
        .will_return("slice", Ret::Array(slice.clone()));

    assert_eq!(
        slice,
        persister.slice(&collection, 1, Some(2)).await.unwrap()
    );
    f.collection_persister.recorder.assert_once_with(
        "slice",
        &[
            CollectionPersisterMock::collection(&collection),
            Arg::Value(Value::Int(1)),
            Arg::Value(Value::Int(2)),
        ],
    );
}

pub async fn testInvokeContains(f: &Fixture) {
    let entity = slc::state("Foo");
    let element = slc::state("Bar");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);

    f.register_managed(&entity);

    f.collection_persister
        .recorder
        .will_return("contains", Ret::Bool(false));

    assert!(!persister.contains(&collection, &element).await.unwrap());
    f.collection_persister.recorder.assert_once_with(
        "contains",
        &[
            CollectionPersisterMock::collection(&collection),
            Arg::object(&element),
        ],
    );
}

pub async fn testInvokeContainsKey(f: &Fixture) {
    let entity = slc::state("Foo");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);

    f.register_managed(&entity);

    f.collection_persister
        .recorder
        .will_return("containsKey", Ret::Bool(false));

    assert!(
        !persister
            .contains_key(&collection, &Key::Int(0))
            .await
            .unwrap()
    );
    f.collection_persister.recorder.assert_once_with(
        "containsKey",
        &[
            CollectionPersisterMock::collection(&collection),
            Arg::Value(Value::Int(0)),
        ],
    );
}

pub async fn testInvokeGet(f: &Fixture) {
    let entity = slc::state("Foo");
    let element = slc::state("Bar");
    let persister = f.create_persister_default();
    let collection = f.create_collection(&entity);

    f.register_managed(&entity);

    f.collection_persister
        .recorder
        .will_return("get", Ret::Object(Some(element.clone())));

    let got = persister
        .get(&collection, &Value::Int(0))
        .await
        .unwrap()
        .unwrap();
    assert!(std::ptr::addr_eq(Arc::as_ptr(&element), Arc::as_ptr(&got)));
    f.collection_persister.recorder.assert_once_with(
        "get",
        &[
            CollectionPersisterMock::collection(&collection),
            Arg::Value(Value::Int(0)),
        ],
    );
}
