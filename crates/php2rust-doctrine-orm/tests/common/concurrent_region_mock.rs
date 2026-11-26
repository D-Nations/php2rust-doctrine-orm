//! `Doctrine\Tests\Mocks\ConcurrentRegionMock`: a `ConcurrentRegion`
//! over a region, which records its calls (by method name, the key's
//! hash or nothing), throws the exceptions queued for a method, and
//! keeps the locks by key hash.
//!
//! Upstream: doctrine/orm tests/Tests/Mocks/ConcurrentRegionMock.php

#![allow(dead_code, reason = "each test file uses a subset")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use php_value::Thrown;
use php2rust_doctrine_orm::cache::exception::lock_exception;
use php2rust_doctrine_orm::cache::region::MultipleEntries;
use php2rust_doctrine_orm::cache::{
    CacheEntry, CacheKey, CollectionCacheEntry, ConcurrentRegion, Lock, Region,
};

#[derive(Debug, Default)]
struct MockState {
    /// `$calls[$method][]`: the key hash of each call (empty without a
    /// key).
    calls: HashMap<&'static str, Vec<String>>,
    /// `$exceptions[$method]`.
    exceptions: HashMap<&'static str, Vec<Option<Thrown>>>,
    /// `$locks[$key->hash]`.
    locks: HashMap<String, Lock>,
}

/// `new ConcurrentRegionMock($region)`.
#[derive(Debug)]
pub struct ConcurrentRegionMock {
    region: Arc<dyn Region>,
    state: Mutex<MockState>,
}

impl ConcurrentRegionMock {
    pub fn new(region: Arc<dyn Region>) -> Self {
        Self {
            region,
            state: Mutex::default(),
        }
    }

    fn state(&self) -> MutexGuard<'_, MockState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn call(&self, method: &'static str, key: Option<&CacheKey>) {
        self.state()
            .calls
            .entry(method)
            .or_default()
            .push(key.map_or_else(String::new, |key| key.hash().to_owned()));
    }

    /// Dequeue an exception for a specific method invocation.
    fn throw_exception(&self, method: &'static str) -> Result<(), Thrown> {
        let exception = {
            let mut state = self.state();
            match state.exceptions.get_mut(method) {
                Some(queue) if !queue.is_empty() => queue.remove(0),
                _ => None,
            }
        };
        match exception {
            Some(exception) => Err(exception),
            None => Ok(()),
        }
    }

    /// Queue an exception for the next method invocation.
    pub fn add_exception(&self, method: &'static str, e: Thrown) {
        self.state()
            .exceptions
            .entry(method)
            .or_default()
            .push(Some(e));
    }

    /// Locks a specific cache entry.
    pub fn set_lock(&self, key: &CacheKey, lock: Lock) {
        self.state().locks.insert(key.hash().to_owned(), lock);
    }

    /// `count($this->calls[$method])`.
    pub fn calls(&self, method: &str) -> usize {
        self.state().calls.get(method).map_or(0, Vec::len)
    }

    fn is_locked(&self, key: &CacheKey) -> bool {
        self.state().locks.contains_key(key.hash())
    }
}

impl Region for ConcurrentRegionMock {
    fn get_name(&self) -> String {
        self.call("getName", None);
        // `getName()` returns a string: an exception cannot be thrown here.
        self.region.get_name()
    }

    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.call("contains", Some(key));
        if self.is_locked(key) {
            return Ok(false);
        }
        self.throw_exception("contains")?;
        self.region.contains(key)
    }

    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        self.call("get", Some(key));
        self.throw_exception("get")?;
        if self.is_locked(key) {
            return Ok(None);
        }
        self.region.get(key)
    }

    fn get_multiple(
        &self,
        collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown> {
        self.call("getMultiple", None);
        self.throw_exception("getMultiple")?;
        self.region.get_multiple(collection)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry, lock: Option<&Lock>) -> Result<bool, Thrown> {
        self.call("put", Some(key));
        self.throw_exception("put")?;
        let held = self.state().locks.get(key.hash()).cloned();
        if let Some(held) = held {
            if let Some(lock) = lock
                && held.value == lock.value
            {
                return self.region.put(key, entry, None);
            }
            return Ok(false);
        }
        self.region.put(key, entry, None)
    }

    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.call("evict", Some(key));
        self.throw_exception("evict")?;
        self.region.evict(key)
    }

    fn evict_all(&self) -> Result<bool, Thrown> {
        self.call("evictAll", None);
        self.throw_exception("evictAll")?;
        self.region.evict_all()
    }

    fn as_concurrent(&self) -> Option<&dyn ConcurrentRegion> {
        Some(self)
    }
}

impl ConcurrentRegion for ConcurrentRegionMock {
    fn lock(&self, key: &CacheKey) -> Result<Option<Lock>, Thrown> {
        self.call("lock", Some(key));
        self.throw_exception("lock")?;
        if self.is_locked(key) {
            return Ok(None);
        }
        let lock = Lock::create_lock_read();
        self.state()
            .locks
            .insert(key.hash().to_owned(), lock.clone());
        Ok(Some(lock))
    }

    fn unlock(&self, key: &CacheKey, lock: &Lock) -> Result<bool, Thrown> {
        self.call("unlock", Some(key));
        self.throw_exception("unlock")?;
        let held = self.state().locks.get(key.hash()).cloned();
        let Some(held) = held else {
            return Ok(false);
        };
        if held.value != lock.value {
            return Err(lock_exception("unexpected lock value"));
        }
        self.state().locks.remove(key.hash());
        Ok(true)
    }
}
