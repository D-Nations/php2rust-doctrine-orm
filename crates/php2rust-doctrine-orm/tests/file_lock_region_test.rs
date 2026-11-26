//! Upstream: doctrine/orm tests/Tests/ORM/Cache/FileLockRegionTest.php
//!
//! `RegionTestCase`'s tests come from `common/cache.rs`. The lock
//! directory is a fresh `doctrine_lock_*` directory under the system's
//! temporary directory, removed when the test ends (`tearDown()`).
//! `getLockFileName()` is public here (upstream reaches it by
//! reflection).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/cache.rs"]
mod cache;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use php_value::Value;
use php2rust_doctrine_orm::cache::pool::ArrayAdapter;
use php2rust_doctrine_orm::cache::{
    ConcurrentRegion, DefaultRegion, FileLockRegion, Region, uniqid,
};

use cache::{entry_mock, key_mock};

/// The region and its directory, which is removed on drop.
struct Fixture {
    region: FileLockRegion,
    directory: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

fn create_region(lock_lifetime: i64) -> Fixture {
    let directory = std::env::temp_dir().join(format!("doctrine_lock_{}", uniqid("")));
    let region = DefaultRegion::new("concurren_region_test", Arc::new(ArrayAdapter::new()), 0);
    Fixture {
        region: FileLockRegion::new(Arc::new(region), &directory, lock_lifetime).unwrap(),
        directory,
    }
}

fn set_up() -> Fixture {
    create_region(60)
}

fn assert_string_equals_file(file: &Path, expected: &str) {
    assert_eq!(expected, std::fs::read_to_string(file).unwrap());
}

#[test]
fn testPutGetContainsEvict() {
    for (key, value) in cache::data_provider_cache_values() {
        let fixture = set_up();
        cache::put_get_contains_evict(&fixture.region, &key, &value);
    }
}

#[test]
fn testEvictAll() {
    let fixture = set_up();
    cache::evict_all(&fixture.region);
}

#[test]
fn testGetRegionName() {
    let fixture = set_up();
    assert_eq!("concurren_region_test", fixture.region.get_name());
}

#[test]
fn testLockAndUnlock() {
    let fixture = set_up();
    let region = &fixture.region;
    let key = key_mock("key");
    let entry = entry_mock(&[("foo", Value::string("bar"))]);
    let file = region.get_lock_file_name(&key);

    assert!(!region.contains(&key).unwrap());
    assert!(region.put(&key, entry, None).unwrap());
    assert!(region.contains(&key).unwrap());

    let lock = region.lock(&key).unwrap();

    assert!(file.exists());
    let lock = lock.expect("assertInstanceOf(Lock::class)");
    assert_string_equals_file(&file, &lock.value);

    // should be not available after lock
    assert!(!region.contains(&key).unwrap());
    assert!(region.get(&key).unwrap().is_none());

    assert!(region.unlock(&key, &lock).unwrap());
    assert!(!file.exists());
}

#[test]
fn testLockWithExistingLock() {
    let fixture = set_up();
    let region = &fixture.region;
    let key = key_mock("key");
    let entry = entry_mock(&[("foo", Value::string("bar"))]);
    let file = region.get_lock_file_name(&key);

    assert!(!region.contains(&key).unwrap());
    assert!(region.put(&key, entry, None).unwrap());
    assert!(region.contains(&key).unwrap());

    std::fs::write(&file, "foo").unwrap();
    assert!(file.exists());
    assert_string_equals_file(&file, "foo");

    assert!(region.lock(&key).unwrap().is_none());
    assert_string_equals_file(&file, "foo");
    assert!(file.exists());

    // should be not available
    assert!(!region.contains(&key).unwrap());
    assert!(region.get(&key).unwrap().is_none());
}

#[test]
fn testUnlockWithExistingLock() {
    let fixture = set_up();
    let region = &fixture.region;
    let key = key_mock("key");
    let entry = entry_mock(&[("foo", Value::string("bar"))]);
    let file = region.get_lock_file_name(&key);

    assert!(!region.contains(&key).unwrap());
    assert!(region.put(&key, entry, None).unwrap());
    assert!(region.contains(&key).unwrap());

    let lock = region
        .lock(&key)
        .unwrap()
        .expect("assertInstanceOf(Lock::class)");
    assert_string_equals_file(&file, &lock.value);
    assert!(file.exists());

    // change the lock
    std::fs::write(&file, "foo").unwrap();
    assert!(file.exists());
    assert_string_equals_file(&file, "foo");

    //try to unlock
    assert!(!region.unlock(&key, &lock).unwrap());
    assert_string_equals_file(&file, "foo");
    assert!(file.exists());

    // should be not available
    assert!(!region.contains(&key).unwrap());
    assert!(region.get(&key).unwrap().is_none());
}

#[test]
fn testPutWithExistingLock() {
    let fixture = set_up();
    let region = &fixture.region;
    let key = key_mock("key");
    let entry = entry_mock(&[("foo", Value::string("bar"))]);
    let file = region.get_lock_file_name(&key);

    assert!(!region.contains(&key).unwrap());
    assert!(region.put(&key, entry.clone(), None).unwrap());
    assert!(region.contains(&key).unwrap());

    // create lock
    std::fs::write(&file, "foo").unwrap();
    assert!(file.exists());
    assert_string_equals_file(&file, "foo");

    assert!(!region.contains(&key).unwrap());
    assert!(!region.put(&key, entry, None).unwrap());
    assert!(!region.contains(&key).unwrap());

    assert!(file.exists());
    assert_string_equals_file(&file, "foo");
}

#[test]
fn testLockedEvict() {
    let fixture = set_up();
    let region = &fixture.region;
    let key = key_mock("key");
    let entry = entry_mock(&[("foo", Value::string("bar"))]);
    let file = region.get_lock_file_name(&key);

    assert!(!region.contains(&key).unwrap());
    assert!(region.put(&key, entry, None).unwrap());
    assert!(region.contains(&key).unwrap());

    let lock = region
        .lock(&key)
        .unwrap()
        .expect("assertInstanceOf(Lock::class)");
    assert_string_equals_file(&file, &lock.value);
    assert!(file.exists());

    assert!(!region.contains(&key).unwrap());
    assert!(region.evict(&key).unwrap());
    assert!(!region.contains(&key).unwrap());
    assert!(!file.exists());
}

#[test]
fn testLockedEvictAll() {
    let fixture = set_up();
    let region = &fixture.region;
    let key1 = key_mock("key1");
    let entry1 = entry_mock(&[("foo1", Value::string("bar1"))]);
    let file1 = region.get_lock_file_name(&key1);

    let key2 = key_mock("key2");
    let entry2 = entry_mock(&[("foo2", Value::string("bar2"))]);
    let file2 = region.get_lock_file_name(&key2);

    assert!(!region.contains(&key1).unwrap());
    assert!(region.put(&key1, entry1, None).unwrap());
    assert!(region.contains(&key1).unwrap());

    assert!(!region.contains(&key2).unwrap());
    assert!(region.put(&key2, entry2, None).unwrap());
    assert!(region.contains(&key2).unwrap());

    let lock1 = region
        .lock(&key1)
        .unwrap()
        .expect("assertInstanceOf(Lock::class)");
    let lock2 = region
        .lock(&key2)
        .unwrap()
        .expect("assertInstanceOf(Lock::class)");

    assert_string_equals_file(&file2, &lock2.value);
    assert_string_equals_file(&file1, &lock1.value);

    assert!(file1.exists());
    assert!(file2.exists());

    assert!(region.evict_all().unwrap());

    assert!(!file1.exists());
    assert!(!file2.exists());

    assert!(!region.contains(&key1).unwrap());
    assert!(!region.contains(&key2).unwrap());
}

#[test]
fn testLockLifetime() {
    let fixture = create_region(-10);
    let region = &fixture.region;
    let key = key_mock("key");
    let entry = entry_mock(&[("foo", Value::string("bar"))]);
    let file = region.get_lock_file_name(&key);

    assert!(!region.contains(&key).unwrap());
    assert!(region.put(&key, entry, None).unwrap());
    assert!(region.contains(&key).unwrap());

    let lock = region
        .lock(&key)
        .unwrap()
        .expect("assertInstanceOf(Lock::class)");
    assert_string_equals_file(&file, &lock.value);
    assert!(file.exists());

    // outdated lock should be removed
    assert!(region.contains(&key).unwrap());
    assert!(region.get(&key).unwrap().is_some());
    assert!(!file.exists());
}
