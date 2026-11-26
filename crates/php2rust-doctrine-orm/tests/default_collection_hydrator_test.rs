//! Upstream: doctrine/orm tests/Tests/ORM/Cache/DefaultCollectionHydratorTest.php
//!
//! The test's entity manager is `slc_entity_manager()` (on SQLite in
//! memory, no schema: the test reads the cache only).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/slc.rs"]
mod slc;

use std::sync::Arc;

use php_value::{Key, PhpArray, Value};
use php2rust_doctrine_collections::ArrayCollection;
use php2rust_doctrine_orm::cache::{
    CollectionCacheEntry, CollectionCacheKey, CollectionHydrator, DefaultCollectionHydrator,
    EntityCacheEntry, EntityCacheKey,
};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_NEW};

fn set_up() -> (slc::Slc, DefaultCollectionHydrator) {
    let slc = slc::slc_entity_manager();
    let structure = DefaultCollectionHydrator::new(&slc.em);
    (slc, structure)
}

#[test]
fn testImplementsCollectionEntryStructure() {
    let (_slc, structure) = set_up();
    let _: &dyn CollectionHydrator = &structure;
}

#[tokio::test]
async fn testLoadCacheCollection() {
    let (slc, structure) = set_up();
    let em = &slc.em;
    let target_region = em
        .get_cache()
        .unwrap()
        .get_entity_cache_region(slc::CITY)
        .unwrap()
        .unwrap();
    let entry = CollectionCacheEntry::list([
        EntityCacheKey::new(slc::CITY, &slc::id(31)).into(),
        EntityCacheKey::new(slc::CITY, &slc::id(32)).into(),
    ]);

    target_region
        .put(
            &EntityCacheKey::new(slc::CITY, &slc::id(31)).into(),
            EntityCacheEntry::new(
                slc::CITY,
                slc::array(&[("id", Value::Int(31)), ("name", Value::string("Foo"))]),
            )
            .into(),
            None,
        )
        .unwrap();
    target_region
        .put(
            &EntityCacheKey::new(slc::CITY, &slc::id(32)).into(),
            EntityCacheEntry::new(
                slc::CITY,
                slc::array(&[("id", Value::Int(32)), ("name", Value::string("Bar"))]),
            )
            .into(),
            None,
        )
        .unwrap();

    let source_class = em.get_class_metadata(slc::STATE).unwrap();
    let target_class = em.get_class_metadata(slc::CITY).unwrap();
    let source_name = source_class.read().name.clone();
    let target_name = target_class.read().name.clone();
    let key = CollectionCacheKey::new(source_name, "cities", &slc::id(21));
    let collection = PersistentCollection::new(
        Some(em),
        Some(Arc::clone(&target_class)),
        ArrayCollection::new(PhpArray::new()),
    );
    let list = structure
        .load_cache_entry(&source_class, &key, &entry, &collection)
        .await
        .unwrap();

    let list = list.unwrap();
    let elements = collection.unwrap();
    assert_eq!(2, list.len());
    assert_eq!(2, elements.len());

    let object = |array: &PhpArray, index: i64| match array.get(&Key::Int(index)) {
        Some(Value::Object(object)) => object.clone(),
        other => panic!("not an object: {other:?}"),
    };
    let (list0, list1) = (object(&list, 0), object(&list, 1));
    let (coll0, coll1) = (object(&elements, 0), object(&elements, 1));

    assert_eq!(target_name, list0.class());
    assert_eq!(target_name, list1.class());
    assert_eq!(target_name, coll0.class());
    assert_eq!(target_name, coll1.class());

    assert!(Arc::ptr_eq(&list0, &coll0));
    assert!(Arc::ptr_eq(&list1, &coll1));

    assert_eq!(Value::Int(31), orm::get(&list0, "id"));
    assert_eq!(Value::Int(32), orm::get(&list1, "id"));
    assert_eq!(orm::get(&list0, "id"), orm::get(&coll0, "id"));
    assert_eq!(orm::get(&list1, "id"), orm::get(&coll1, "id"));
    let uow = em.get_unit_of_work();
    assert_eq!(STATE_MANAGED, uow.get_entity_state_or(&coll0, STATE_NEW));
    assert_eq!(STATE_MANAGED, uow.get_entity_state_or(&coll1, STATE_NEW));
}
