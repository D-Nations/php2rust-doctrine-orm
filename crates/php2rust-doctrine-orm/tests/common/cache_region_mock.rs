//! `Doctrine\Tests\Mocks\CacheRegionMock` and `TimestampRegionMock`:
//! regions that record each call (its key and entry) and return the
//! values queued for a method, or the method's default.
//!
//! `getMultiple()` returns the entries a test queued; an index without
//! an entry is one the PHP array holds as `false` or `null`.
//!
//! Upstream: doctrine/orm tests/Tests/Mocks/CacheRegionMock.php
//! Upstream: doctrine/orm tests/Tests/Mocks/TimestampRegionMock.php

#![allow(dead_code, reason = "each test file uses a subset")]

use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard, PoisonError};

use php_value::Thrown;
use php2rust_doctrine_orm::cache::region::MultipleEntries;
use php2rust_doctrine_orm::cache::{
    CacheEntry, CacheKey, CollectionCacheEntry, Lock, Region, TimestampRegion,
};

/// A call: its key and entry, when it has them.
#[derive(Debug, Clone)]
pub struct Call {
    pub key: Option<CacheKey>,
    pub entry: Option<CacheEntry>,
}

/// A value queued with `addReturn()`.
#[derive(Debug, Clone)]
pub enum Return {
    Bool(bool),
    Entry(Option<CacheEntry>),
    Multiple(Option<MultipleEntries>),
}

#[derive(Debug, Default)]
struct MockState {
    calls: HashMap<&'static str, Vec<Call>>,
    returns: HashMap<&'static str, VecDeque<Return>>,
}

/// `new CacheRegionMock()`.
#[derive(Debug, Default)]
pub struct CacheRegionMock {
    state: Mutex<MockState>,
    /// `$name`.
    pub name: String,
}

impl CacheRegionMock {
    pub fn new() -> Self {
        Self {
            state: Mutex::default(),
            name: "mock".to_owned(),
        }
    }

    fn state(&self) -> MutexGuard<'_, MockState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn call(&self, method: &'static str, key: Option<&CacheKey>, entry: Option<&CacheEntry>) {
        self.state().calls.entry(method).or_default().push(Call {
            key: key.cloned(),
            entry: entry.cloned(),
        });
    }

    /// Queue a return value for a specific method invocation.
    pub fn add_return(&self, method: &'static str, value: Return) {
        self.state()
            .returns
            .entry(method)
            .or_default()
            .push_back(value);
    }

    /// Dequeue a value for a specific method invocation.
    fn get_return(&self, method: &'static str) -> Option<Return> {
        self.state()
            .returns
            .get_mut(method)
            .and_then(VecDeque::pop_front)
    }

    fn get_bool(&self, method: &'static str, default: bool) -> bool {
        match self.get_return(method) {
            Some(Return::Bool(value)) => value,
            _ => default,
        }
    }

    /// `$this->calls[$method]`, `None` when the key is not set.
    pub fn calls(&self, method: &str) -> Option<Vec<Call>> {
        self.state().calls.get(method).cloned()
    }

    /// `clear()`.
    pub fn clear(&self) {
        let mut state = self.state();
        state.calls.clear();
        state.returns.clear();
    }
}

impl Region for CacheRegionMock {
    fn get_name(&self) -> String {
        self.call("getName", None, None);
        self.name.clone()
    }

    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.call("contains", Some(key), None);
        Ok(self.get_bool("contains", false))
    }

    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        self.call("get", Some(key), None);
        Ok(match self.get_return("get") {
            Some(Return::Entry(entry)) => entry,
            _ => None,
        })
    }

    fn get_multiple(
        &self,
        _collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown> {
        self.call("getMultiple", None, None);
        Ok(match self.get_return("getMultiple") {
            Some(Return::Multiple(entries)) => entries,
            _ => None,
        })
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry, _lock: Option<&Lock>) -> Result<bool, Thrown> {
        self.call("put", Some(key), Some(&entry));
        Ok(self.get_bool("put", true))
    }

    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.call("evict", Some(key), None);
        Ok(self.get_bool("evict", true))
    }

    fn evict_all(&self) -> Result<bool, Thrown> {
        self.call("evictAll", None, None);
        Ok(self.get_bool("evictAll", true))
    }
}

/// `new TimestampRegionMock()`.
#[derive(Debug, Default)]
pub struct TimestampRegionMock {
    pub region: CacheRegionMock,
}

impl TimestampRegionMock {
    pub fn new() -> Self {
        Self {
            region: CacheRegionMock::new(),
        }
    }
}

impl Region for TimestampRegionMock {
    fn get_name(&self) -> String {
        self.region.get_name()
    }

    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.region.contains(key)
    }

    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        self.region.get(key)
    }

    fn get_multiple(
        &self,
        collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown> {
        self.region.get_multiple(collection)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry, lock: Option<&Lock>) -> Result<bool, Thrown> {
        self.region.put(key, entry, lock)
    }

    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.region.evict(key)
    }

    fn evict_all(&self) -> Result<bool, Thrown> {
        self.region.evict_all()
    }

    fn as_timestamp(&self) -> Option<&dyn TimestampRegion> {
        Some(self)
    }
}

impl TimestampRegion for TimestampRegionMock {
    fn update(&self, key: &CacheKey) -> Result<(), Thrown> {
        self.region.call("update", Some(key), None);
        Ok(())
    }
}
