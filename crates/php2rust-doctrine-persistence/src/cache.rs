//! The PSR-6 cache interfaces (`Psr\Cache\CacheItemPoolInterface`,
//! `CacheItemInterface`) that `AbstractClassMetadataFactory::setCache()`
//! takes, and [`ArrayCachePool`], an in-memory pool like symfony/cache's
//! `ArrayAdapter` that the tests use.
//!
//! A cached value is any object (`Arc<dyn Any + Send + Sync>`); PHP pools
//! store a serialized copy, [`ArrayCachePool`] stores the shared value.
//!
//! Upstream: none

use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use php_value::Thrown;

/// A cached value.
pub type CacheValue = Arc<dyn Any + Send + Sync>;

/// `Psr\Cache\CacheItemInterface`: an item, which `set()` changes in place.
pub trait CacheItem: Send + Sync {
    /// The key for the current cache item.
    fn get_key(&self) -> String;

    /// The value of the item from the cache associated with this object's
    /// key (`None` for `null`, which a miss gives).
    fn get(&self) -> Option<CacheValue>;

    /// Confirms if the cache item lookup resulted in a cache hit.
    fn is_hit(&self) -> bool;

    /// Sets the value represented by this cache item.
    fn set(&self, value: Option<CacheValue>);
}

/// `Psr\Cache\CacheItemPoolInterface`.
pub trait CacheItemPool: Send + Sync {
    /// Returns a Cache Item representing the specified key.
    fn get_item(&self, key: &str) -> Result<Arc<dyn CacheItem>, Thrown>;

    /// Returns a traversable set of cache items.
    fn get_items(&self, keys: &[String]) -> Result<Vec<Arc<dyn CacheItem>>, Thrown>;

    /// Confirms if the cache contains specified cache item.
    fn has_item(&self, key: &str) -> Result<bool, Thrown>;

    /// Deletes all items in the pool.
    fn clear(&self) -> bool;

    /// Removes the item from the pool.
    fn delete_item(&self, key: &str) -> Result<bool, Thrown>;

    /// Removes multiple items from the pool.
    fn delete_items(&self, keys: &[String]) -> Result<bool, Thrown>;

    /// Persists a cache item immediately.
    fn save(&self, item: Arc<dyn CacheItem>) -> bool;

    /// Sets a cache item to be persisted later.
    fn save_deferred(&self, item: Arc<dyn CacheItem>) -> bool;

    /// Persists any deferred cache items.
    fn commit(&self) -> bool;
}

/// symfony/cache's `CacheItem`: a key, a value and whether it was a hit.
pub struct ArrayCacheItem {
    key: String,
    state: Mutex<(Option<CacheValue>, bool)>,
}

impl fmt::Debug for ArrayCacheItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArrayCacheItem")
            .field("key", &self.key)
            .field("is_hit", &self.is_hit())
            .finish_non_exhaustive()
    }
}

impl ArrayCacheItem {
    fn new(key: &str, value: Option<CacheValue>, hit: bool) -> Self {
        Self {
            key: key.to_owned(),
            state: Mutex::new((value, hit)),
        }
    }
}

impl CacheItem for ArrayCacheItem {
    fn get_key(&self) -> String {
        self.key.clone()
    }

    fn get(&self) -> Option<CacheValue> {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .0
            .clone()
    }

    fn is_hit(&self) -> bool {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).1
    }

    fn set(&self, value: Option<CacheValue>) {
        self.state.lock().unwrap_or_else(PoisonError::into_inner).0 = value;
    }
}

/// An in-memory pool, as symfony/cache's `ArrayAdapter` (no expiry).
#[derive(Default)]
pub struct ArrayCachePool {
    values: Mutex<HashMap<String, Option<CacheValue>>>,
    deferred: Mutex<Vec<(String, Option<CacheValue>)>>,
}

impl fmt::Debug for ArrayCachePool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values = self.values.lock().unwrap_or_else(PoisonError::into_inner);
        let mut keys: Vec<&String> = values.keys().collect();
        keys.sort();
        f.debug_struct("ArrayCachePool")
            .field("keys", &keys)
            .finish_non_exhaustive()
    }
}

/// The characters PSR-6 reserves in keys.
const RESERVED_CHARACTERS: &str = "{}()/\\@:";

fn validate_key(key: &str) -> Result<(), Thrown> {
    if key.is_empty() {
        return Err(Thrown::new(
            "Symfony\\Component\\Cache\\Exception\\InvalidArgumentException",
            "Cache key length must be greater than zero.",
            0,
        ));
    }
    if key.chars().any(|c| RESERVED_CHARACTERS.contains(c)) {
        return Err(Thrown::new(
            "Symfony\\Component\\Cache\\Exception\\InvalidArgumentException",
            format!("Cache key \"{key}\" contains reserved characters \"{RESERVED_CHARACTERS}\"."),
            0,
        ));
    }
    Ok(())
}

impl ArrayCachePool {
    pub fn new() -> Self {
        Self::default()
    }

    fn store(&self, key: String, value: Option<CacheValue>) {
        self.values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, value);
    }
}

impl CacheItemPool for ArrayCachePool {
    fn get_item(&self, key: &str) -> Result<Arc<dyn CacheItem>, Thrown> {
        validate_key(key)?;
        self.commit();
        let values = self.values.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(Arc::new(match values.get(key) {
            Some(value) => ArrayCacheItem::new(key, value.clone(), true),
            None => ArrayCacheItem::new(key, None, false),
        }))
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<Arc<dyn CacheItem>>, Thrown> {
        keys.iter().map(|key| self.get_item(key)).collect()
    }

    fn has_item(&self, key: &str) -> Result<bool, Thrown> {
        validate_key(key)?;
        self.commit();
        Ok(self
            .values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(key))
    }

    fn clear(&self) -> bool {
        self.values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        self.deferred
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        true
    }

    fn delete_item(&self, key: &str) -> Result<bool, Thrown> {
        validate_key(key)?;
        self.values
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(key);
        Ok(true)
    }

    fn delete_items(&self, keys: &[String]) -> Result<bool, Thrown> {
        for key in keys {
            self.delete_item(key)?;
        }
        Ok(true)
    }

    fn save(&self, item: Arc<dyn CacheItem>) -> bool {
        self.store(item.get_key(), item.get());
        true
    }

    fn save_deferred(&self, item: Arc<dyn CacheItem>) -> bool {
        self.deferred
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((item.get_key(), item.get()));
        true
    }

    fn commit(&self) -> bool {
        let deferred =
            std::mem::take(&mut *self.deferred.lock().unwrap_or_else(PoisonError::into_inner));
        for (key, value) in deferred {
            self.store(key, value);
        }
        true
    }
}
