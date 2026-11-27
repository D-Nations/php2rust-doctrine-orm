//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheConcurrentTest.php
//!
//! `CacheFactorySecondLevelCacheConcurrentTest` builds its persisters as
//! the PHP subclass inherits them (`factory::build_*`) and keeps each
//! `ConcurrentRegionMock` it returns by region name:
//! `assert($region instanceof ConcurrentRegionMock)` is that lookup. The
//! Country metadata is changed in place (each test has its own entity
//! manager, so `tearDown()` restores nothing).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/cache_region_mock.rs"]
mod cache_region_mock;
#[path = "common/concurrent_region_mock.rs"]
mod concurrent_region_mock;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_orm::cache::factory;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::cache::{
    Cache, CacheFactory, CacheKey, CachedCollectionPersister, CachedEntityPersister,
    CollectionCacheKey, CollectionHydrator, DefaultCacheFactory, DefaultRegion, EntityCacheKey,
    EntityHydrator, Lock, QueryCache, Region, TimestampRegion,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::CACHE_USAGE_NONSTRICT_READ_WRITE;
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata};
use php2rust_doctrine_orm::persisters::EntityPersisterInterface;
use php2rust_doctrine_orm::persisters::collection::CollectionPersister;

use cache_region_mock::TimestampRegionMock;
use concurrent_region_mock::ConcurrentRegionMock;
use slc_functional::{COUNTRY, Fixture, STATE, assert_instance_of, count, id, name};

/// `CacheFactorySecondLevelCacheConcurrentTest`.
#[derive(Debug)]
struct CacheFactorySecondLevelCacheConcurrentTest {
    inner: Arc<DefaultCacheFactory>,
    cache: Arc<ArrayAdapter>,
    /// The regions returned by `getRegion()`, by name.
    regions: Mutex<HashMap<String, Arc<ConcurrentRegionMock>>>,
}

impl CacheFactorySecondLevelCacheConcurrentTest {
    /// The `ConcurrentRegionMock` named `$name`.
    fn region(&self, name: &str) -> Arc<ConcurrentRegionMock> {
        self.regions.lock().unwrap().get(name).cloned().unwrap()
    }
}

impl CacheFactory for CacheFactorySecondLevelCacheConcurrentTest {
    fn build_cached_entity_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn EntityPersisterInterface>,
        metadata: Arc<ClassMetadata>,
    ) -> Result<Arc<dyn CachedEntityPersister>, Thrown> {
        factory::build_cached_entity_persister(self, em, persister, metadata)
    }

    fn build_cached_collection_persister(
        &self,
        em: &Arc<EntityManager>,
        persister: Arc<dyn CollectionPersister>,
        mapping: &AssociationMapping,
    ) -> Result<Arc<dyn CachedCollectionPersister>, Thrown> {
        factory::build_cached_collection_persister(self, em, persister, mapping)
    }

    fn build_query_cache(
        &self,
        em: &Arc<EntityManager>,
        region_name: Option<&str>,
    ) -> Result<Arc<dyn QueryCache>, Thrown> {
        factory::build_query_cache(self, em, region_name)
    }

    fn build_collection_hydrator(
        &self,
        em: &Arc<EntityManager>,
        mapping: &AssociationMapping,
    ) -> Arc<dyn CollectionHydrator> {
        self.inner.build_collection_hydrator(em, mapping)
    }

    fn build_entity_hydrator(
        &self,
        em: &Arc<EntityManager>,
        metadata: &Arc<ClassMetadata>,
    ) -> Arc<dyn EntityHydrator> {
        self.inner.build_entity_hydrator(em, metadata)
    }

    fn get_region(&self, cache: &PhpArray) -> Result<Arc<dyn Region>, Thrown> {
        let name = match cache.get(&Key::from("region")) {
            Some(Value::String(name)) => name.to_string(),
            _ => String::new(),
        };
        let mut regions = self.regions.lock().unwrap();
        let region = regions
            .entry(name.clone())
            .or_insert_with(|| {
                Arc::new(ConcurrentRegionMock::new(Arc::new(DefaultRegion::new(
                    name,
                    self.cache.clone(),
                    0,
                ))))
            })
            .clone();
        Ok(region)
    }

    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion> {
        Arc::new(TimestampRegionMock::new())
    }

    fn create_cache(&self, em: &Arc<EntityManager>) -> Result<Arc<dyn Cache>, Thrown> {
        self.inner.create_cache(em)
    }
}

/// `setUp()`.
async fn set_up() -> (Fixture, Arc<CacheFactorySecondLevelCacheConcurrentTest>) {
    let f = slc_functional::set_up().await;
    let cache_factory = Arc::new(CacheFactorySecondLevelCacheConcurrentTest {
        inner: f.factory.clone(),
        cache: f.pool.clone(),
        regions: Mutex::default(),
    });
    f.em.get_configuration()
        .get_second_level_cache_configuration()
        .unwrap()
        .set_cache_factory(cache_factory.clone());

    let country_metadata = f.em.get_class_metadata(COUNTRY).unwrap();
    let mut cache = country_metadata.read().cache.clone().unwrap_or_default();
    cache.insert("usage", Value::Int(CACHE_USAGE_NONSTRICT_READ_WRITE));
    country_metadata.write().cache = Some(cache);
    (f, cache_factory)
}

fn id_array(id: Value) -> PhpArray {
    let mut identifier = PhpArray::new();
    identifier.insert("id", id);
    identifier
}

#[tokio::test]
async fn testBasicConcurrentEntityReadLock() {
    let (mut f, cache_factory) = set_up().await;
    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    let country_id = id(&f.countries[0]);
    let cache_id = CacheKey::from(EntityCacheKey::new(COUNTRY, &id_array(country_id.clone())));
    let region = f.cache.get_entity_cache_region(COUNTRY).unwrap().unwrap();
    let region = cache_factory.region(&region.get_name());

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));

    region.set_lock(&cache_id, Lock::create_lock_read()); // another proc lock the entity cache

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));

    f.reset_query_log();
    let country = f.find(COUNTRY, country_id).await.unwrap();

    assert_instance_of(COUNTRY, &country);
    f.assert_query_count(1);
    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
}

#[tokio::test]
async fn testBasicConcurrentCollectionReadLock() {
    let (mut f, cache_factory) = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.em.clear().unwrap();
    f.evict_regions();

    let state_id = id(&f.states[0]);
    let state = f.find(STATE, state_id.clone()).await.unwrap();

    assert_instance_of(STATE, &state);
    let country = f.load_one(&state, "country").await;
    assert_instance_of(COUNTRY, &country);
    assert!(!name(&country).is_null());
    assert_eq!(2, count(&state, "cities").await);

    f.em.clear().unwrap();
    f.logger.clear_stats();

    let cache_id = CacheKey::from(CollectionCacheKey::new(
        STATE,
        "cities",
        &id_array(state_id.clone()),
    ));
    let region = f
        .cache
        .get_collection_cache_region(STATE, "cities")
        .unwrap()
        .unwrap();
    let region = cache_factory.region(&region.get_name());

    assert!(f.contains_collection(STATE, "cities", &f.states[0]));

    region.set_lock(&cache_id, Lock::create_lock_read()); // another proc lock the entity cache

    assert!(!f.contains_collection(STATE, "cities", &f.states[0]));

    f.reset_query_log();
    let state = f.find(STATE, state_id).await.unwrap();

    assert_eq!(0, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_hit_count());

    assert_eq!(
        0,
        f.logger.get_region_miss_count(&f.get_entity_region(STATE))
    );
    assert_eq!(
        1,
        f.logger.get_region_hit_count(&f.get_entity_region(STATE))
    );

    assert_instance_of(STATE, &state);
    assert_eq!(2, count(&state, "cities").await);

    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_hit_count());
    assert_eq!(
        1,
        f.logger
            .get_region_miss_count(&f.get_collection_region(STATE, "cities"))
    );

    f.assert_query_count(1);
    assert!(!f.contains_collection(STATE, "cities", &f.states[0]));
}
