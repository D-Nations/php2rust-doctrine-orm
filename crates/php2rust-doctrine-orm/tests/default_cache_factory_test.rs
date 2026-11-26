//! Upstream: doctrine/orm tests/Tests/ORM/Cache/DefaultCacheFactoryTest.php
//!
//! The `DefaultCacheFactory` mock that replaces `getRegion()` is
//! `FactoryMock`: it builds its persisters with
//! `factory::build_cached_entity_persister()` and
//! `factory::build_cached_collection_persister()`, as the PHP subclass
//! inherits them, records each `getRegion()` and returns the test's
//! region. `assertInstanceOf()` of a persister class is its
//! `get_class_name()`; `clone $metadata` is a new `ClassMetadata` of the
//! same data. `createMock(CacheItemPoolInterface::class)` is an
//! `ArrayAdapter`. `expectExceptionMessage()` is a substring check.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/concurrent_region_mock.rs"]
mod concurrent_region_mock;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/slc.rs"]
mod slc;

use std::any::Any;
use std::sync::{Arc, Mutex};

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::cache::collection_persister::{
    NON_STRICT_READ_WRITE_CACHED_COLLECTION_PERSISTER, READ_ONLY_CACHED_COLLECTION_PERSISTER,
    READ_WRITE_CACHED_COLLECTION_PERSISTER,
};
use php2rust_doctrine_orm::cache::factory;
use php2rust_doctrine_orm::cache::persister::{
    NON_STRICT_READ_WRITE_CACHED_ENTITY_PERSISTER, READ_ONLY_CACHED_ENTITY_PERSISTER,
    READ_WRITE_CACHED_ENTITY_PERSISTER,
};
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::cache::{
    Cache, CacheFactory, CachedCollectionPersister, CachedEntityPersister, CollectionHydrator,
    DefaultCacheFactory, DefaultRegion, EntityHydrator, QueryCache, Region, RegionsConfiguration,
    TimestampRegion,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::{
    CACHE_USAGE_NONSTRICT_READ_WRITE, CACHE_USAGE_READ_ONLY, CACHE_USAGE_READ_WRITE,
};
use php2rust_doctrine_orm::mapping::{AssociationMapping, ClassMetadata};
use php2rust_doctrine_orm::persisters::BasicEntityPersister;
use php2rust_doctrine_orm::persisters::EntityPersisterInterface;
use php2rust_doctrine_orm::persisters::collection::{
    CollectionPersister, DefaultCollectionPersister,
};

use concurrent_region_mock::ConcurrentRegionMock;

const ATTRACTION_CONTACT_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\AttractionContactInfo";
const ATTRACTION_LOCATION_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\AttractionLocationInfo";

/// `getMockBuilder(DefaultCacheFactory::class)->onlyMethods(['getRegion'])`.
#[derive(Debug)]
struct FactoryMock {
    inner: DefaultCacheFactory,
    /// The `$cache` of each `getRegion()` call.
    calls: Mutex<Vec<PhpArray>>,
    /// `->willReturn($region)`.
    region: Mutex<Option<Arc<dyn Region>>>,
}

impl FactoryMock {
    fn will_return(&self, region: Arc<dyn Region>) {
        *self.region.lock().unwrap() = Some(region);
    }

    /// `expects(self::once())->method('getRegion')->with(self::equalTo($cache))`.
    fn assert_get_region_once_with(&self, cache: &PhpArray) {
        let calls = self.calls.lock().unwrap();
        assert_eq!(1, calls.len());
        assert_eq!(cache, &calls[0]);
    }
}

impl CacheFactory for FactoryMock {
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
        self.calls.lock().unwrap().push(cache.clone());
        Ok(self
            .region
            .lock()
            .unwrap()
            .clone()
            // A mock's method returns a stub of its return type.
            .unwrap_or_else(|| slc::RegionMock::new()))
    }

    fn get_timestamp_region(&self) -> Arc<dyn TimestampRegion> {
        self.inner.get_timestamp_region()
    }

    fn create_cache(&self, em: &Arc<EntityManager>) -> Result<Arc<dyn Cache>, Thrown> {
        self.inner.create_cache(em)
    }
}

struct Fixture {
    slc: slc::Slc,
    regions_config: Arc<RegionsConfiguration>,
    factory: FactoryMock,
}

impl Fixture {
    fn em(&self) -> &Arc<EntityManager> {
        &self.slc.em
    }

    /// `getSharedSecondLevelCache()`.
    fn pool(&self) -> Arc<ArrayAdapter> {
        Arc::clone(&self.slc.pool)
    }

    /// `new DefaultCacheFactory($this->regionsConfig, $this->getSharedSecondLevelCache())`.
    fn default_factory(&self) -> DefaultCacheFactory {
        DefaultCacheFactory::new(Arc::clone(&self.regions_config), self.pool())
    }

    /// `new ConcurrentRegionMock(new DefaultRegion('regionName', $this->getSharedSecondLevelCache()))`.
    fn region(&self) -> Arc<dyn Region> {
        Arc::new(ConcurrentRegionMock::new(Arc::new(DefaultRegion::new(
            "regionName",
            self.pool(),
            0,
        ))))
    }

    /// `clone $em->getClassMetadata($class)`.
    fn cloned_metadata(&self, class: &str) -> Arc<ClassMetadata> {
        let metadata = self.em().get_class_metadata(class).unwrap();
        Arc::new(ClassMetadata::from_info(metadata.read().clone()))
    }

    /// `$metadata->associationMappings['cities']` of `State`.
    fn cities(&self) -> AssociationMapping {
        self.em()
            .get_class_metadata(slc::STATE)
            .unwrap()
            .read()
            .association_mappings
            .get("cities")
            .cloned()
            .unwrap()
    }

    /// `new BasicEntityPersister($em, $metadata)`.
    fn persister(&self, metadata: &Arc<ClassMetadata>) -> Arc<dyn EntityPersisterInterface> {
        Arc::new(BasicEntityPersister::new(self.em(), Arc::clone(metadata)))
    }

    /// `new OneToManyPersister($em)`.
    fn one_to_many_persister(&self) -> Arc<dyn CollectionPersister> {
        Arc::new(DefaultCollectionPersister::new(self.em(), true))
    }
}

/// `setUp()`.
fn set_up() -> Fixture {
    let slc = slc::slc_entity_manager();
    let regions_config = Arc::new(RegionsConfiguration::default());
    let factory = FactoryMock {
        inner: DefaultCacheFactory::new(Arc::clone(&regions_config), slc.pool.clone()),
        calls: Mutex::default(),
        region: Mutex::default(),
    };
    Fixture {
        slc,
        regions_config,
        factory,
    }
}

fn set_usage(cache: &mut Option<PhpArray>, usage: i64) -> PhpArray {
    let cache = cache.get_or_insert_with(PhpArray::new);
    cache.insert("usage", Value::Int(usage));
    cache.clone()
}

fn same<T: ?Sized, U: ?Sized>(a: &Arc<T>, b: &Arc<U>) -> bool {
    std::ptr::addr_eq(Arc::as_ptr(a), Arc::as_ptr(b))
}

fn build_entity_persister(usage: i64, class_name: &str) {
    let f = set_up();
    let em = f.em();
    let metadata = f.cloned_metadata(slc::STATE);
    let persister = f.persister(&metadata);
    let region = f.region();

    let cache = set_usage(&mut metadata.write().cache, usage);

    f.factory.will_return(region);

    let cached_persister = f
        .factory
        .build_cached_entity_persister(em, persister, metadata)
        .unwrap();

    f.factory.assert_get_region_once_with(&cache);
    assert_eq!(class_name, cached_persister.get_class_name());
}

fn build_collection_persister(usage: i64, class_name: &str) {
    let f = set_up();
    let em = f.em();
    let mut mapping = f.cities();
    let persister = f.one_to_many_persister();
    let region = f.region();

    let cache = set_usage(&mut mapping.cache, usage);

    f.factory.will_return(region);

    let cached_persister = f
        .factory
        .build_cached_collection_persister(em, persister, &mapping)
        .unwrap();

    f.factory.assert_get_region_once_with(&cache);
    assert_eq!(class_name, cached_persister.get_class_name());
}

#[test]
fn testImplementsCacheFactory() {
    let f = set_up();
    let _: &dyn CacheFactory = &f.factory;
}

#[test]
fn testBuildCachedEntityPersisterReadOnly() {
    build_entity_persister(CACHE_USAGE_READ_ONLY, READ_ONLY_CACHED_ENTITY_PERSISTER);
}

#[test]
fn testBuildCachedEntityPersisterReadWrite() {
    build_entity_persister(CACHE_USAGE_READ_WRITE, READ_WRITE_CACHED_ENTITY_PERSISTER);
}

#[test]
fn testBuildCachedEntityPersisterNonStrictReadWrite() {
    build_entity_persister(
        CACHE_USAGE_NONSTRICT_READ_WRITE,
        NON_STRICT_READ_WRITE_CACHED_ENTITY_PERSISTER,
    );
}

#[test]
fn testBuildCachedCollectionPersisterReadOnly() {
    build_collection_persister(CACHE_USAGE_READ_ONLY, READ_ONLY_CACHED_COLLECTION_PERSISTER);
}

#[test]
fn testBuildCachedCollectionPersisterReadWrite() {
    build_collection_persister(
        CACHE_USAGE_READ_WRITE,
        READ_WRITE_CACHED_COLLECTION_PERSISTER,
    );
}

#[test]
fn testBuildCachedCollectionPersisterNonStrictReadWrite() {
    build_collection_persister(
        CACHE_USAGE_NONSTRICT_READ_WRITE,
        NON_STRICT_READ_WRITE_CACHED_COLLECTION_PERSISTER,
    );
}

#[test]
fn testInheritedEntityCacheRegion() {
    let f = set_up();
    let em = f.em();
    let metadata1 = f.cloned_metadata(ATTRACTION_CONTACT_INFO);
    let metadata2 = f.cloned_metadata(ATTRACTION_LOCATION_INFO);
    let persister1 = f.persister(&metadata1);
    let persister2 = f.persister(&metadata2);
    let factory = f.default_factory();

    let cached_persister1 = factory
        .build_cached_entity_persister(em, persister1, metadata1)
        .unwrap();
    let cached_persister2 = factory
        .build_cached_entity_persister(em, persister2, metadata2)
        .unwrap();

    assert!(!same(&cached_persister1, &cached_persister2));
    assert!(same(
        &cached_persister1.get_cache_region(),
        &cached_persister2.get_cache_region()
    ));
}

#[test]
fn testCreateNewCacheDriver() {
    let f = set_up();
    let em = f.em();
    let metadata1 = f.cloned_metadata(slc::STATE);
    let metadata2 = f.cloned_metadata(slc::CITY);
    let persister1 = f.persister(&metadata1);
    let persister2 = f.persister(&metadata2);
    let factory = f.default_factory();

    let cached_persister1 = factory
        .build_cached_entity_persister(em, persister1, metadata1)
        .unwrap();
    let cached_persister2 = factory
        .build_cached_entity_persister(em, persister2, metadata2)
        .unwrap();

    assert!(!same(&cached_persister1, &cached_persister2));
    assert!(!same(
        &cached_persister1.get_cache_region(),
        &cached_persister2.get_cache_region()
    ));
}

#[test]
fn testBuildCachedEntityPersisterNonStrictException() {
    let f = set_up();
    let em = f.em();
    let metadata = f.cloned_metadata(slc::STATE);
    let persister = f.persister(&metadata);

    set_usage(&mut metadata.write().cache, -1);

    let thrown = f
        .factory
        .build_cached_entity_persister(em, persister, metadata)
        .err()
        .unwrap();
    assert_eq!("InvalidArgumentException", thrown.class);
    assert_eq!(
        "Unrecognized access strategy type [-1]",
        thrown.message.to_string()
    );
}

#[test]
fn testBuildCachedCollectionPersisterException() {
    let f = set_up();
    let em = f.em();
    let mut mapping = f.cities();
    let persister = f.one_to_many_persister();

    set_usage(&mut mapping.cache, -1);

    f.factory.will_return(f.region());
    let thrown = f
        .factory
        .build_cached_collection_persister(em, persister, &mapping)
        .err()
        .unwrap();
    assert_eq!("InvalidArgumentException", thrown.class);
    assert_eq!(
        "Unrecognized access strategy type [-1]",
        thrown.message.to_string()
    );
}

const LOCK_DIRECTORY_MESSAGE: &str = "If you want to use a \"READ_WRITE\" cache an implementation of \"Doctrine\\ORM\\Cache\\ConcurrentRegion\" is required, The default implementation provided by doctrine is \"Doctrine\\ORM\\Cache\\Region\\FileLockRegion\" if you want to use it please provide a valid directory";

fn read_write_foo() -> PhpArray {
    slc::array(&[
        ("usage", Value::Int(CACHE_USAGE_READ_WRITE)),
        ("region", Value::string("foo")),
    ])
}

#[test]
fn testInvalidFileLockRegionDirectoryException() {
    let f = set_up();
    let factory = f.default_factory();

    let thrown = factory.get_region(&read_write_foo()).err().unwrap();
    assert_eq!("LogicException", thrown.class);
    assert!(thrown.message.to_string().contains(LOCK_DIRECTORY_MESSAGE));
}

#[test]
fn testInvalidFileLockRegionDirectoryExceptionWithEmptyString() {
    let f = set_up();
    let factory = f.default_factory();

    factory.set_file_lock_region_directory("");

    let thrown = factory.get_region(&read_write_foo()).err().unwrap();
    assert_eq!("LogicException", thrown.class);
    assert!(thrown.message.to_string().contains(LOCK_DIRECTORY_MESSAGE));
}

#[test]
fn testBuildsDefaultCacheRegionFromGenericCacheRegion() {
    let f = set_up();
    let factory =
        DefaultCacheFactory::new(Arc::clone(&f.regions_config), Arc::new(ArrayAdapter::new()));

    let region = factory
        .get_region(&slc::array(&[
            ("region", Value::string("bar")),
            ("usage", Value::Int(CACHE_USAGE_READ_ONLY)),
        ]))
        .unwrap();
    assert!(
        (region.as_ref() as &dyn Any)
            .downcast_ref::<DefaultRegion>()
            .is_some()
    );
}
