//! The cache regions: the `Region`, `ConcurrentRegion` and
//! `TimestampRegion` interfaces, `DefaultRegion` over a PSR-6 pool,
//! `UpdateTimestampCache`, and `FileLockRegion`, which locks a key with a
//! file in a directory.
//!
//! `FileLockRegion` reads a lock's time from the file's access time, as
//! `fileatime()` does.
//!
//! Upstream: doctrine/orm src/Cache/Region.php src/Cache/ConcurrentRegion.php
//! Upstream: doctrine/orm src/Cache/TimestampRegion.php src/Cache/Region/DefaultRegion.php
//! Upstream: doctrine/orm src/Cache/Region/UpdateTimestampCache.php
//! Upstream: doctrine/orm src/Cache/Region/FileLockRegion.php

use std::any::Any;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use php_value::{Key, Thrown};

use super::exception::invalid_argument;
use super::pool::{CacheItemPool, PoolValue};
use super::{CacheEntry, CacheKey, CollectionCacheEntry, Lock, TimestampCacheEntry};

/// The entries of `getMultiple()`, under the collection's keys.
pub type MultipleEntries = Vec<(Key, CacheEntry)>;

/// `Doctrine\ORM\Cache\Region`: a contract for a cache region.
pub trait Region: Debug + Send + Sync + Any {
    /// The name of this region.
    fn get_name(&self) -> String;

    /// Determine whether this region contains data for the given key.
    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown>;

    /// Get an item from the cache.
    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown>;

    /// Get all items from the cache identified by an array of keys.
    fn get_multiple(
        &self,
        collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown>;

    /// Put an item into the cache.
    fn put(&self, key: &CacheKey, entry: CacheEntry, lock: Option<&Lock>) -> Result<bool, Thrown>;

    /// Remove an item from the cache.
    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown>;

    /// Remove all contents of this particular cache region.
    fn evict_all(&self) -> Result<bool, Thrown>;

    /// The region as a `ConcurrentRegion`, if it is one.
    fn as_concurrent(&self) -> Option<&dyn ConcurrentRegion> {
        None
    }

    /// The region as a `TimestampRegion`, if it is one.
    fn as_timestamp(&self) -> Option<&dyn TimestampRegion> {
        None
    }
}

/// `Doctrine\ORM\Cache\ConcurrentRegion`: a region that supports locking.
pub trait ConcurrentRegion: Region {
    /// Attempts to read lock the mapping for the given key; `None` when
    /// the key is locked.
    fn lock(&self, key: &CacheKey) -> Result<Option<Lock>, Thrown>;

    /// Attempts to read unlock the mapping for the given key.
    fn unlock(&self, key: &CacheKey, lock: &Lock) -> Result<bool, Thrown>;
}

/// `Doctrine\ORM\Cache\TimestampRegion`: a region that keeps the
/// timestamps of the last updates.
pub trait TimestampRegion: Region {
    /// Update a specific key into the cache region.
    fn update(&self, key: &CacheKey) -> Result<(), Thrown>;
}

const REGION_KEY_SEPARATOR: &str = "_";
const REGION_PREFIX: &str = "DC2_REGION_";

/// `Doctrine\ORM\Cache\Region\DefaultRegion`: a region over a PSR-6
/// cache item pool.
#[derive(Debug)]
pub struct DefaultRegion {
    name: String,
    cache_item_pool: Arc<dyn CacheItemPool>,
    lifetime: i64,
}

impl DefaultRegion {
    /// `new DefaultRegion($name, $cacheItemPool, $lifetime)`.
    pub fn new(
        name: impl Into<String>,
        cache_item_pool: Arc<dyn CacheItemPool>,
        lifetime: i64,
    ) -> Self {
        Self {
            name: name.into(),
            cache_item_pool,
            lifetime,
        }
    }

    fn get_cache_entry_key(&self, key: &CacheKey) -> String {
        let hash: String = key
            .hash()
            .chars()
            .map(|c| if "{}()/\\@:".contains(c) { '_' } else { c })
            .collect();
        format!("{REGION_PREFIX}{}{REGION_KEY_SEPARATOR}{hash}", self.name)
    }
}

impl Region for DefaultRegion {
    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.cache_item_pool
            .has_item(&self.get_cache_entry_key(key))
    }

    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        let item = self
            .cache_item_pool
            .get_item(&self.get_cache_entry_key(key))?;
        Ok(match item.get() {
            Some(PoolValue::Entry(entry)) if item.is_hit() => Some(entry.clone()),
            _ => None,
        })
    }

    fn get_multiple(
        &self,
        collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown> {
        let keys: Vec<String> = collection
            .identifiers
            .iter()
            .map(|(_, key)| self.get_cache_entry_key(key))
            .collect();
        let items = self.cache_item_pool.get_items(&keys)?;
        let mut result = Vec::with_capacity(keys.len());
        for ((array_key, _), cache_key) in collection.identifiers.iter().zip(&keys) {
            let Some(item) = items.iter().find(|item| item.get_key() == cache_key) else {
                return Ok(None);
            };
            match item.get() {
                Some(PoolValue::Entry(entry)) if item.is_hit() => {
                    result.push((array_key.clone(), entry.clone()));
                }
                _ => return Ok(None),
            }
        }
        Ok(Some(result))
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry, _lock: Option<&Lock>) -> Result<bool, Thrown> {
        let mut item = self
            .cache_item_pool
            .get_item(&self.get_cache_entry_key(key))?
            .set(PoolValue::Entry(entry));
        if self.lifetime > 0 {
            item = item.expires_after(self.lifetime);
        }
        Ok(self.cache_item_pool.save(item))
    }

    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown> {
        self.cache_item_pool
            .delete_item(&self.get_cache_entry_key(key))
    }

    fn evict_all(&self) -> Result<bool, Thrown> {
        Ok(self
            .cache_item_pool
            .clear(&format!("{REGION_PREFIX}{}", self.name)))
    }
}

/// `Doctrine\ORM\Cache\Region\UpdateTimestampCache`: a `DefaultRegion`
/// that keeps a `TimestampCacheEntry` of now on `update()`.
#[derive(Debug)]
pub struct UpdateTimestampCache {
    region: DefaultRegion,
}

impl UpdateTimestampCache {
    pub fn new(
        name: impl Into<String>,
        cache_item_pool: Arc<dyn CacheItemPool>,
        lifetime: i64,
    ) -> Self {
        Self {
            region: DefaultRegion::new(name, cache_item_pool, lifetime),
        }
    }
}

impl Region for UpdateTimestampCache {
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

impl TimestampRegion for UpdateTimestampCache {
    fn update(&self, key: &CacheKey) -> Result<(), Thrown> {
        self.put(key, TimestampCacheEntry::new(None).into(), None)?;
        Ok(())
    }
}

/// `FileLockRegion::LOCK_EXTENSION`.
pub const LOCK_EXTENSION: &str = "lock";

/// `Doctrine\ORM\Cache\Region\FileLockRegion`: a concurrent region that
/// locks a key with a file in a directory.
#[derive(Debug)]
pub struct FileLockRegion {
    region: Arc<dyn Region>,
    directory: PathBuf,
    lock_lifetime: i64,
}

impl FileLockRegion {
    /// `new FileLockRegion($region, $directory, $lockLifetime)`: creates
    /// the directory when it does not exist.
    pub fn new(
        region: Arc<dyn Region>,
        directory: impl Into<PathBuf>,
        lock_lifetime: i64,
    ) -> Result<Self, Thrown> {
        let directory = directory.into();
        if !directory.is_dir() && std::fs::create_dir_all(&directory).is_err() {
            return Err(invalid_argument(format!(
                "The directory \"{}\" does not exist and could not be created.",
                directory.display()
            )));
        }
        let writable = std::fs::metadata(&directory).is_ok_and(|m| !m.permissions().readonly());
        if !writable {
            return Err(invalid_argument(format!(
                "The directory \"{}\" is not writable.",
                directory.display()
            )));
        }
        Ok(Self {
            region,
            directory,
            lock_lifetime,
        })
    }

    /// `getLockFileName()`: `{directory}/{hash}.lock`.
    pub fn get_lock_file_name(&self, key: &CacheKey) -> PathBuf {
        self.directory
            .join(format!("{}.{LOCK_EXTENSION}", key.hash()))
    }

    fn get_lock_time(filename: &Path) -> Option<i64> {
        let accessed = std::fs::metadata(filename).ok()?.accessed().ok()?;
        let seconds = accessed.duration_since(UNIX_EPOCH).ok()?.as_secs();
        i64::try_from(seconds).ok()
    }

    fn is_locked(&self, key: &CacheKey, lock: Option<&Lock>) -> bool {
        let filename = self.get_lock_file_name(key);
        if !filename.is_file() {
            return false;
        }
        let time = Self::get_lock_time(&filename);
        let content = std::fs::read(&filename).ok();
        let (Some(time), Some(content)) = (time, content) else {
            let _ = std::fs::remove_file(&filename);
            return false;
        };
        if lock.is_some_and(|lock| content == lock.value.as_bytes()) {
            return false;
        }
        // outdated lock
        if time.saturating_add(self.lock_lifetime) <= php_value::clock::time() {
            let _ = std::fs::remove_file(&filename);
            return false;
        }
        true
    }
}

impl Region for FileLockRegion {
    fn get_name(&self) -> String {
        self.region.get_name()
    }

    fn contains(&self, key: &CacheKey) -> Result<bool, Thrown> {
        if self.is_locked(key, None) {
            return Ok(false);
        }
        self.region.contains(key)
    }

    fn get(&self, key: &CacheKey) -> Result<Option<CacheEntry>, Thrown> {
        if self.is_locked(key, None) {
            return Ok(None);
        }
        self.region.get(key)
    }

    fn get_multiple(
        &self,
        collection: &CollectionCacheEntry,
    ) -> Result<Option<MultipleEntries>, Thrown> {
        if collection
            .identifiers
            .iter()
            .any(|(_, key)| self.is_locked(key, None))
        {
            return Ok(None);
        }
        self.region.get_multiple(collection)
    }

    fn put(&self, key: &CacheKey, entry: CacheEntry, lock: Option<&Lock>) -> Result<bool, Thrown> {
        if self.is_locked(key, lock) {
            return Ok(false);
        }
        self.region.put(key, entry, None)
    }

    fn evict(&self, key: &CacheKey) -> Result<bool, Thrown> {
        if self.is_locked(key, None) {
            let _ = std::fs::remove_file(self.get_lock_file_name(key));
        }
        self.region.evict(key)
    }

    fn evict_all(&self) -> Result<bool, Thrown> {
        if let Ok(entries) = std::fs::read_dir(&self.directory) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == LOCK_EXTENSION) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        self.region.evict_all()
    }

    fn as_concurrent(&self) -> Option<&dyn ConcurrentRegion> {
        Some(self)
    }
}

impl ConcurrentRegion for FileLockRegion {
    fn lock(&self, key: &CacheKey) -> Result<Option<Lock>, Thrown> {
        if self.is_locked(key, None) {
            return Ok(None);
        }
        let lock = Lock::create_lock_read();
        let filename = self.get_lock_file_name(key);
        if std::fs::write(&filename, lock.value.as_bytes()).is_err() {
            return Ok(None);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&filename, std::fs::Permissions::from_mode(0o664));
        }
        Ok(Some(lock))
    }

    fn unlock(&self, key: &CacheKey, lock: &Lock) -> Result<bool, Thrown> {
        if self.is_locked(key, Some(lock)) {
            return Ok(false);
        }
        Ok(std::fs::remove_file(self.get_lock_file_name(key)).is_ok())
    }
}
