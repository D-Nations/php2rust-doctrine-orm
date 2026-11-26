//! Port of tests/Mapping/ClassMetadataFactoryTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6.
//! symfony/cache's `ArrayAdapter` is the crate's `ArrayCachePool`; the
//! PSR-6 mocks are `MockCacheItem` and `MockCachePool`. A cached metadata
//! value is an `Arc` of the factory's metadata.
//!
//! Upstream: doctrine/persistence tests/Mapping/ClassMetadataFactoryTest.php

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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use common::{MockMappingDriver, StubClassMetadata, TestClassMetadataFactory, assert_thrown, same};
use php_value::Thrown;
use php_value::object::StdClass;
use php2rust_doctrine_persistence::cache::{ArrayCachePool, CacheItem, CacheItemPool, CacheValue};
use php2rust_doctrine_persistence::mapping::{AbstractClassMetadataFactory, mapping_exception};
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

const ROOT_ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\RootEntity";
const CHILD_ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\ChildEntity";
const FOO: &str = "Foo";
const STD_CLASS: &str = "stdClass";

type Factory = TestClassMetadataFactory<StubClassMetadata>;

fn set_up() -> Factory {
    common::register_fixtures();
    register_class(ClassInfo::class(ROOT_ENTITY));
    register_class(ClassInfo::class(CHILD_ENTITY).extends(ROOT_ENTITY));
    let driver = MockMappingDriver::stub();
    let metadata = StubClassMetadata::new();
    TestClassMetadataFactory::new(driver, metadata)
}

/// The cache value of a metadata.
fn cached(metadata: &Arc<StubClassMetadata>) -> CacheValue {
    Arc::new(Arc::clone(metadata))
}

fn as_metadata(value: &CacheValue) -> Arc<StubClassMetadata> {
    value
        .downcast_ref::<Arc<StubClassMetadata>>()
        .cloned()
        .unwrap()
}

#[test]
fn testSetGetCache() {
    let cmf = set_up();
    assert!(cmf.get_cache().is_none());

    let cache: Arc<dyn CacheItemPool> = Arc::new(ArrayCachePool::new());
    cmf.set_cache(Arc::clone(&cache));

    assert!(same(&cache, &cmf.get_cache().unwrap()));
}

#[test]
fn testGetMetadataFor() {
    let cmf = set_up();
    let _metadata = cmf.get_metadata_for(STD_CLASS).unwrap();

    assert!(cmf.has_metadata_for(STD_CLASS));
}

#[test]
fn testGetMetadataForAbsentClass() {
    let cmf = set_up();
    assert_thrown(cmf.get_metadata_for(FOO), mapping_exception::CLASS, "");
}

#[test]
fn testGetParentMetadata() {
    let cmf = set_up();
    let _metadata = cmf.get_metadata_for(CHILD_ENTITY).unwrap();

    assert!(cmf.has_metadata_for(CHILD_ENTITY));
    assert!(cmf.has_metadata_for(ROOT_ENTITY));
}

#[test]
fn testGetCachedMetadata() {
    let cmf = set_up();
    let metadata = StubClassMetadata::new();
    let cache = Arc::new(ArrayCachePool::new());
    let item = cache.get_item(&cmf.get_cache_key(CHILD_ENTITY)).unwrap();
    item.set(Some(cached(&metadata)));
    cache.save(item);

    cmf.set_cache(cache);

    assert_eq!(*metadata, *cmf.get_metadata_for(CHILD_ENTITY).unwrap());
}

#[test]
fn testCacheGetMetadataFor() {
    let cmf = set_up();
    let cache = Arc::new(ArrayCachePool::new());
    cmf.set_cache(Arc::clone(&cache) as Arc<dyn CacheItemPool>);

    let loaded_metadata = cmf.get_metadata_for(CHILD_ENTITY).unwrap();

    let item = cache.get_item(&cmf.get_cache_key(CHILD_ENTITY)).unwrap();
    assert!(item.is_hit());
    assert_eq!(*loaded_metadata, *as_metadata(&item.get().unwrap()));
}

#[test]
fn testWillFallbackOnNotLoadedMetadata() {
    let cmf = set_up();
    let class_metadata = StubClassMetadata::new();

    let fallback = Arc::clone(&class_metadata);
    cmf.set_fallback_callback(move || Some(Arc::clone(&fallback)));

    assert!(same(&class_metadata, &cmf.get_metadata_for(FOO).unwrap()));
}

#[test]
fn testWillFailOnFallbackFailureWithNotLoadedMetadata() {
    let cmf = set_up();
    cmf.set_fallback_callback(|| None);

    assert_thrown(
        cmf.get_metadata_for(FOO),
        mapping_exception::CLASS,
        "Class 'Foo' does not exist",
    );
}

/// `$this->createMock(CacheItemInterface::class)`.
struct MockCacheItem {
    key: String,
    value: Option<CacheValue>,
    set_calls: Mutex<Vec<Option<CacheValue>>>,
}

impl CacheItem for MockCacheItem {
    fn get_key(&self) -> String {
        self.key.clone()
    }

    fn get(&self) -> Option<CacheValue> {
        self.value.clone()
    }

    fn is_hit(&self) -> bool {
        false
    }

    fn set(&self, value: Option<CacheValue>) {
        self.set_calls.lock().unwrap().push(value);
    }
}

/// `$this->createMock(CacheItemPoolInterface::class)`.
struct MockCachePool {
    item: Arc<MockCacheItem>,
    get_item_calls: Mutex<Vec<String>>,
    get_items_calls: Mutex<Vec<Vec<String>>>,
    save_deferred_calls: AtomicUsize,
    commit_calls: AtomicUsize,
}

impl MockCachePool {
    fn new(item: Arc<MockCacheItem>) -> Arc<Self> {
        Arc::new(Self {
            item,
            get_item_calls: Mutex::new(Vec::new()),
            get_items_calls: Mutex::new(Vec::new()),
            save_deferred_calls: AtomicUsize::new(0),
            commit_calls: AtomicUsize::new(0),
        })
    }
}

impl CacheItemPool for MockCachePool {
    fn get_item(&self, key: &str) -> Result<Arc<dyn CacheItem>, Thrown> {
        self.get_item_calls.lock().unwrap().push(key.to_owned());
        Ok(self.item.clone())
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<Arc<dyn CacheItem>>, Thrown> {
        self.get_items_calls.lock().unwrap().push(keys.to_vec());
        Ok(vec![self.item.clone()])
    }

    fn has_item(&self, _key: &str) -> Result<bool, Thrown> {
        Ok(false)
    }

    fn clear(&self) -> bool {
        false
    }

    fn delete_item(&self, _key: &str) -> Result<bool, Thrown> {
        Ok(false)
    }

    fn delete_items(&self, _keys: &[String]) -> Result<bool, Thrown> {
        Ok(false)
    }

    fn save(&self, _item: Arc<dyn CacheItem>) -> bool {
        false
    }

    fn save_deferred(&self, item: Arc<dyn CacheItem>) -> bool {
        let expected: Arc<dyn CacheItem> = self.item.clone();
        assert!(same(&expected, &item));
        self.save_deferred_calls.fetch_add(1, Ordering::SeqCst);
        false
    }

    fn commit(&self) -> bool {
        self.commit_calls.fetch_add(1, Ordering::SeqCst);
        false
    }
}

#[test]
fn testWillIgnoreCacheEntriesThatAreNotMetadataInstances() {
    let cmf = set_up();
    let key = cmf.get_cache_key(ROOT_ENTITY);

    let metadata = Arc::clone(&cmf.metadata);

    let item = Arc::new(MockCacheItem {
        key: key.clone(),
        value: Some(Arc::new(StdClass::default())),
        set_calls: Mutex::new(Vec::new()),
    });

    let cache_driver = MockCachePool::new(Arc::clone(&item));

    cmf.set_cache(Arc::clone(&cache_driver) as Arc<dyn CacheItemPool>);

    assert!(same(&metadata, &cmf.get_metadata_for(ROOT_ENTITY).unwrap()));

    // ->with($key)
    assert!(
        cache_driver
            .get_item_calls
            .lock()
            .unwrap()
            .iter()
            .all(|k| *k == key)
    );
    // getItems() once, with [$key]
    assert_eq!(*cache_driver.get_items_calls.lock().unwrap(), [vec![key]]);
    // $item->expects(self::once())->method('set')->with($metadata)
    let set_calls = item.set_calls.lock().unwrap();
    assert_eq!(set_calls.len(), 1);
    assert!(same(
        &metadata,
        &as_metadata(set_calls[0].as_ref().unwrap())
    ));
    assert_eq!(cache_driver.save_deferred_calls.load(Ordering::SeqCst), 1);
    assert_eq!(cache_driver.commit_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn testWillNotCacheFallbackMetadata() {
    let cmf = set_up();
    let key = cmf.get_cache_key("Foo");

    let metadata = Arc::clone(&cmf.metadata);

    let item = Arc::new(MockCacheItem {
        key: String::new(),
        value: None,
        set_calls: Mutex::new(Vec::new()),
    });

    let cache_driver = MockCachePool::new(Arc::clone(&item));

    cmf.set_cache(Arc::clone(&cache_driver) as Arc<dyn CacheItemPool>);
    let fallback = Arc::clone(&metadata);
    cmf.set_fallback_callback(move || Some(Arc::clone(&fallback)));

    assert!(same(&metadata, &cmf.get_metadata_for(FOO).unwrap()));

    // getItem() once, with $key
    assert_eq!(*cache_driver.get_item_calls.lock().unwrap(), [key]);
    assert!(item.set_calls.lock().unwrap().is_empty());
    assert_eq!(cache_driver.save_deferred_calls.load(Ordering::SeqCst), 0);
    assert_eq!(cache_driver.commit_calls.load(Ordering::SeqCst), 0);
}
