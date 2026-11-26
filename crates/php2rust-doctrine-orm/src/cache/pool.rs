//! A PSR-6 cache item pool, in memory, where upstream's tests and
//! `DefaultCacheFactory` take a `Psr\Cache\CacheItemPoolInterface` (the
//! tests use `Symfony\Component\Cache\Adapter\ArrayAdapter`).
//!
//! `ArrayAdapter` keeps each value as a copy (it stores serialized
//! values by default), checks the keys as `CacheItem::validateKey()`
//! does, expires items by `microtime(true)`, and `clear($prefix)` drops
//! the items whose key starts with the prefix, and the expired ones.
//!
//! Upstream: none

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Mutex, MutexGuard, PoisonError};

use php_value::{Thrown, Value};

use super::CacheEntry;
use super::exception::PSR_INVALID_ARGUMENT_EXCEPTION;

/// The characters a PSR-6 key must not contain.
pub const RESERVED_CHARACTERS: &str = "{}()/\\@:";

/// What a pool item holds: a cache entry, or another PHP value.
#[derive(Debug, Clone, PartialEq)]
pub enum PoolValue {
    Entry(CacheEntry),
    Value(Value),
}

/// `Psr\Cache\CacheItemInterface`.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheItem {
    key: String,
    value: Option<PoolValue>,
    is_hit: bool,
    /// `expiresAfter()`, in seconds.
    lifetime: Option<i64>,
}

impl CacheItem {
    pub fn get_key(&self) -> &str {
        &self.key
    }

    pub fn is_hit(&self) -> bool {
        self.is_hit
    }

    /// `get()`: the value, or `None` for a miss.
    pub fn get(&self) -> Option<&PoolValue> {
        self.value.as_ref()
    }

    /// `set($value)`.
    #[must_use]
    pub fn set(mut self, value: PoolValue) -> Self {
        self.value = Some(value);
        self
    }

    /// `expiresAfter($seconds)`.
    #[must_use]
    pub fn expires_after(mut self, seconds: i64) -> Self {
        self.lifetime = Some(seconds);
        self
    }
}

/// `Psr\Cache\CacheItemPoolInterface`, as far as the second-level cache
/// uses it.
pub trait CacheItemPool: Debug + Send + Sync {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown>;

    /// `getItems($keys)`: the items under their keys, in order.
    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown>;

    fn has_item(&self, key: &str) -> Result<bool, Thrown>;

    /// `clear($prefix)` (Symfony's `AdapterInterface::clear()`).
    fn clear(&self, prefix: &str) -> bool;

    fn delete_item(&self, key: &str) -> Result<bool, Thrown>;

    fn save(&self, item: CacheItem) -> bool;
}

/// `CacheItem::validateKey()`.
pub fn validate_key(key: &str) -> Result<(), Thrown> {
    if key.is_empty() {
        return Err(Thrown::new(
            PSR_INVALID_ARGUMENT_EXCEPTION,
            "Cache key length must be greater than zero.",
            0,
        ));
    }
    if key.contains(|c| RESERVED_CHARACTERS.contains(c)) {
        return Err(Thrown::new(
            PSR_INVALID_ARGUMENT_EXCEPTION,
            format!("Cache key \"{key}\" contains reserved characters \"{RESERVED_CHARACTERS}\"."),
            0,
        ));
    }
    Ok(())
}

/// `Symfony\Component\Cache\Adapter\ArrayAdapter`.
#[derive(Debug, Default)]
pub struct ArrayAdapter {
    default_lifetime: i64,
    /// Each key's value and expiry (`microtime(true)`; `None` never).
    values: Mutex<HashMap<String, (PoolValue, Option<f64>)>>,
}

impl ArrayAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    /// `new ArrayAdapter($defaultLifetime)`.
    pub fn with_default_lifetime(default_lifetime: i64) -> Self {
        Self {
            default_lifetime,
            values: Mutex::default(),
        }
    }

    fn values(&self) -> MutexGuard<'_, HashMap<String, (PoolValue, Option<f64>)>> {
        self.values.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn live(expiry: Option<f64>, now: f64) -> bool {
        expiry.is_none_or(|expiry| expiry > now)
    }

    fn item(&self, key: &str) -> CacheItem {
        let now = php_value::clock::microtime();
        let value = self
            .values()
            .get(key)
            .filter(|(_, expiry)| Self::live(*expiry, now))
            .map(|(value, _)| value.clone());
        CacheItem {
            key: key.to_owned(),
            is_hit: value.is_some(),
            value,
            lifetime: None,
        }
    }
}

impl CacheItemPool for ArrayAdapter {
    fn get_item(&self, key: &str) -> Result<CacheItem, Thrown> {
        validate_key(key)?;
        Ok(self.item(key))
    }

    fn get_items(&self, keys: &[String]) -> Result<Vec<CacheItem>, Thrown> {
        keys.iter().map(|key| self.get_item(key)).collect()
    }

    fn has_item(&self, key: &str) -> Result<bool, Thrown> {
        validate_key(key)?;
        Ok(self.item(key).is_hit)
    }

    fn clear(&self, prefix: &str) -> bool {
        let mut values = self.values();
        if prefix.is_empty() {
            values.clear();
        } else {
            let now = php_value::clock::microtime();
            values.retain(|key, (_, expiry)| Self::live(*expiry, now) && !key.starts_with(prefix));
        }
        true
    }

    fn delete_item(&self, key: &str) -> Result<bool, Thrown> {
        validate_key(key)?;
        self.values().remove(key);
        Ok(true)
    }

    fn save(&self, item: CacheItem) -> bool {
        let Some(value) = item.value else {
            self.values().remove(&item.key);
            return true;
        };
        let lifetime = item.lifetime.unwrap_or(self.default_lifetime);
        let expiry = if item.lifetime.is_some() || lifetime > 0 {
            #[expect(clippy::cast_precision_loss, reason = "the expiry is a float")]
            Some(php_value::clock::microtime() + lifetime as f64)
        } else {
            None
        };
        if expiry.is_some_and(|expiry| expiry <= php_value::clock::microtime()) {
            self.values().remove(&item.key);
            return true;
        }
        self.values().insert(item.key, (value, expiry));
        true
    }
}
