//! Upstream: doctrine/orm tests/Tests/ORM/Cache/DefaultCacheTest.php
//!
//! `assertInstanceOf()` of an interface is the type; `assertSame()` of
//! two query caches compares the `Arc`s. A collection entry holds cache
//! keys, so the collection entries the tests put are the keys of the
//! cities `['id' => 11]` and `['id' => 12]`. `toIdentifierArray()`
//! (called by reflection upstream) is public.

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::cache::{
    Cache, CollectionCacheEntry, CollectionCacheKey, DefaultCache, EntityCacheEntry,
    EntityCacheKey, QueryCache, Region,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;

const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";

struct Fixture {
    em: Arc<EntityManager>,
    cache: DefaultCache,
    _slc: slc::Slc,
}

/// `setUp()`.
fn set_up() -> Fixture {
    let slc = slc::slc_entity_manager();
    let em = Arc::clone(&slc.em);
    let cache = DefaultCache::new(&em).unwrap();
    Fixture {
        em,
        cache,
        _slc: slc,
    }
}

impl Fixture {
    fn put_entity_cache_entry(&self, class_name: &str, identifier: &PhpArray, data: PhpArray) {
        let metadata = self.em.get_class_metadata(class_name).unwrap();
        let (name, root) = {
            let metadata = metadata.read();
            (metadata.name.clone(), metadata.root_entity_name.clone())
        };
        let cache_key = EntityCacheKey::new(name.as_str(), identifier);
        let cache_entry = EntityCacheEntry::new(name.as_str(), data);
        let persister = self
            .em
            .get_unit_of_work()
            .get_cached_entity_persister(&root)
            .unwrap()
            .unwrap();

        persister
            .get_cache_region()
            .put(&cache_key.into(), cache_entry.into(), None)
            .unwrap();
    }

    fn put_collection_cache_entry(
        &self,
        class_name: &str,
        association: &str,
        owner_identifier: &PhpArray,
        data: CollectionCacheEntry,
    ) {
        let metadata = self.em.get_class_metadata(class_name).unwrap();
        let name = metadata.read().name.clone();
        let mapping = metadata
            .read()
            .get_association_mapping(association)
            .unwrap()
            .clone();
        let cache_key = CollectionCacheKey::new(name, association, owner_identifier);
        let persister = self
            .em
            .get_unit_of_work()
            .get_cached_collection_persister(&mapping)
            .unwrap()
            .unwrap();

        persister
            .get_cache_region()
            .put(&cache_key.into(), data.into(), None)
            .unwrap();
    }
}

fn brazil() -> PhpArray {
    slc::array(&[("id", Value::Int(1)), ("name", Value::string("Brazil"))])
}

/// `[['id' => 11], ['id' => 12]]`, as the keys of the cities.
fn cities_entry() -> CollectionCacheEntry {
    CollectionCacheEntry::list([
        EntityCacheKey::new(slc::CITY, &slc::id(11)).into(),
        EntityCacheKey::new(slc::CITY, &slc::id(12)).into(),
    ])
}

fn implements_cache<T: Cache + ?Sized>(_: &T) {}
fn implements_query_cache<T: QueryCache + ?Sized>(_: &Arc<T>) {}
fn implements_region<T: Region + ?Sized>(_: &Arc<T>) {}

fn same<T: ?Sized>(a: &Arc<T>, b: &Arc<T>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

#[test]
fn testImplementsCache() {
    let f = set_up();
    implements_cache(&f.cache);
}

#[test]
fn testGetEntityCacheRegionAccess() {
    let f = set_up();
    implements_region(
        &f.cache
            .get_entity_cache_region(slc::STATE)
            .unwrap()
            .unwrap(),
    );
    assert!(f.cache.get_entity_cache_region(CMS_USER).unwrap().is_none());
}

#[test]
fn testGetCollectionCacheRegionAccess() {
    let f = set_up();
    implements_region(
        &f.cache
            .get_collection_cache_region(slc::STATE, "cities")
            .unwrap()
            .unwrap(),
    );
    assert!(
        f.cache
            .get_collection_cache_region(CMS_USER, "phonenumbers")
            .unwrap()
            .is_none()
    );
}

#[test]
fn testContainsEntity() {
    let f = set_up();
    let identifier = slc::id(1);
    let class_name = slc::COUNTRY;
    let cache_entry = brazil();

    assert!(
        !f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );

    f.put_entity_cache_entry(class_name, &identifier, cache_entry);

    assert!(
        f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );
    assert!(!f.cache.contains_entity(CMS_USER, &Value::Int(1)).unwrap());
}

#[test]
fn testEvictEntity() {
    let f = set_up();
    let identifier = slc::id(1);
    let class_name = slc::COUNTRY;
    let cache_entry = brazil();

    f.put_entity_cache_entry(class_name, &identifier, cache_entry);

    assert!(
        f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );

    f.cache.evict_entity(slc::COUNTRY, &Value::Int(1)).unwrap();
    f.cache.evict_entity(CMS_USER, &Value::Int(1)).unwrap();

    assert!(
        !f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testEvictEntityRegion() {
    let f = set_up();
    let identifier = slc::id(1);
    let class_name = slc::COUNTRY;
    let cache_entry = brazil();

    f.put_entity_cache_entry(class_name, &identifier, cache_entry);

    assert!(
        f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );

    f.cache.evict_entity_region(slc::COUNTRY).unwrap();
    f.cache.evict_entity_region(CMS_USER).unwrap();

    assert!(
        !f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testEvictEntityRegions() {
    let f = set_up();
    let identifier = slc::id(1);
    let class_name = slc::COUNTRY;
    let cache_entry = brazil();

    f.put_entity_cache_entry(class_name, &identifier, cache_entry);

    assert!(
        f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );

    f.cache.evict_entity_regions().unwrap();

    assert!(
        !f.cache
            .contains_entity(slc::COUNTRY, &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testContainsCollection() {
    let f = set_up();
    let owner_id = slc::id(1);
    let class_name = slc::STATE;
    let association = "cities";

    assert!(
        !f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );

    f.put_collection_cache_entry(class_name, association, &owner_id, cities_entry());

    assert!(
        f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );
    assert!(
        !f.cache
            .contains_collection(CMS_USER, "phonenumbers", &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testEvictCollection() {
    let f = set_up();
    let owner_id = slc::id(1);
    let class_name = slc::STATE;
    let association = "cities";

    f.put_collection_cache_entry(class_name, association, &owner_id, cities_entry());

    assert!(
        f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );

    f.cache
        .evict_collection(class_name, association, &Value::Array(owner_id))
        .unwrap();
    f.cache
        .evict_collection(CMS_USER, "phonenumbers", &Value::Int(1))
        .unwrap();

    assert!(
        !f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testEvictCollectionRegion() {
    let f = set_up();
    let owner_id = slc::id(1);
    let class_name = slc::STATE;
    let association = "cities";

    f.put_collection_cache_entry(class_name, association, &owner_id, cities_entry());

    assert!(
        f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );

    f.cache
        .evict_collection_region(class_name, association)
        .unwrap();
    f.cache
        .evict_collection_region(CMS_USER, "phonenumbers")
        .unwrap();

    assert!(
        !f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testEvictCollectionRegions() {
    let f = set_up();
    let owner_id = slc::id(1);
    let class_name = slc::STATE;
    let association = "cities";

    f.put_collection_cache_entry(class_name, association, &owner_id, cities_entry());

    assert!(
        f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );

    f.cache.evict_collection_regions().unwrap();

    assert!(
        !f.cache
            .contains_collection(slc::STATE, association, &Value::Int(1))
            .unwrap()
    );
}

#[test]
fn testQueryCache() {
    let f = set_up();
    assert!(!f.cache.contains_query("foo"));

    let default_query_cache = f.cache.get_query_cache(None).unwrap();
    let foo_query_cache = f.cache.get_query_cache(Some("foo")).unwrap();

    implements_query_cache(&default_query_cache);
    implements_query_cache(&foo_query_cache);
    assert!(same(
        &default_query_cache,
        &f.cache.get_query_cache(None).unwrap()
    ));
    assert!(same(
        &foo_query_cache,
        &f.cache.get_query_cache(Some("foo")).unwrap()
    ));

    f.cache.evict_query_region(None).unwrap();
    f.cache.evict_query_region(Some("foo")).unwrap();
    f.cache.evict_query_regions().unwrap();

    assert!(f.cache.contains_query("foo"));

    assert!(same(
        &default_query_cache,
        &f.cache.get_query_cache(None).unwrap()
    ));
    assert!(same(
        &foo_query_cache,
        &f.cache.get_query_cache(Some("foo")).unwrap()
    ));
}

#[test]
fn testToIdentifierArrayShouldLookupForEntityIdentifier() {
    let f = set_up();
    let identifier = 123;
    let entity = slc::country("Foo");
    let metadata = f.em.get_class_metadata(slc::COUNTRY).unwrap();

    orm::set(&entity, "id", Value::Int(identifier));

    assert_eq!(
        slc::id(identifier),
        DefaultCache::to_identifier_array(&f.em, &metadata, &Value::Int(identifier)).unwrap()
    );
}
