//! The entries of the second-level cache regions.
//!
//! `AssociationCacheEntry` sits in an `EntityCacheEntry`'s data as a PHP
//! object (it is a `PhpObject`), as it does upstream.
//!
//! Upstream: doctrine/orm src/Cache/CacheEntry.php src/Cache/EntityCacheEntry.php
//! Upstream: doctrine/orm src/Cache/CollectionCacheEntry.php src/Cache/AssociationCacheEntry.php
//! Upstream: doctrine/orm src/Cache/QueryCacheEntry.php src/Cache/TimestampCacheEntry.php

use std::sync::Arc;

use php_value::{Key, PhpArray, PhpObject, Value};

use super::CacheKey;

pub const ASSOCIATION_CACHE_ENTRY: &str = "Doctrine\\ORM\\Cache\\AssociationCacheEntry";

/// `Doctrine\ORM\Cache\EntityCacheEntry`: the entity map data of an
/// entity.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityCacheEntry {
    /// The entity class name.
    pub class: String,
    /// The entity map data.
    pub data: PhpArray,
}

impl EntityCacheEntry {
    pub fn new(class: impl Into<String>, data: PhpArray) -> Self {
        Self {
            class: class.into(),
            data,
        }
    }
}

/// `Doctrine\ORM\Cache\CollectionCacheEntry`: the entity keys a
/// collection holds, under their array keys.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionCacheEntry {
    pub identifiers: Vec<(Key, CacheKey)>,
}

impl CollectionCacheEntry {
    pub fn new(identifiers: Vec<(Key, CacheKey)>) -> Self {
        Self { identifiers }
    }

    /// `new CollectionCacheEntry([$key1, $key2, ...])`: keys under the
    /// list indexes.
    pub fn list(identifiers: impl IntoIterator<Item = CacheKey>) -> Self {
        Self::new(
            (0_i64..)
                .zip(identifiers)
                .map(|(index, key)| (Key::from(index), key))
                .collect(),
        )
    }
}

/// `Doctrine\ORM\Cache\AssociationCacheEntry`: the class and identifier
/// of an associated entity.
#[derive(Debug, Clone, PartialEq)]
pub struct AssociationCacheEntry {
    /// The entity class name.
    pub class: String,
    /// The entity identifier.
    pub identifier: PhpArray,
}

impl AssociationCacheEntry {
    pub fn new(class: impl Into<String>, identifier: PhpArray) -> Self {
        Self {
            class: class.into(),
            identifier,
        }
    }

    /// The entry as a PHP object value.
    pub fn into_value(self) -> Value {
        Value::Object(Arc::new(self))
    }

    /// The entry a value holds, if it is one.
    pub fn from_value(value: &Value) -> Option<&Self> {
        match value {
            Value::Object(object) => object.downcast_ref::<Self>(),
            _ => None,
        }
    }
}

impl PhpObject for AssociationCacheEntry {
    fn class(&self) -> &str {
        ASSOCIATION_CACHE_ENTRY
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("class", Value::string(self.class.as_str()));
        properties.insert("identifier", Value::Array(self.identifier.clone()));
        properties
    }

    fn instance_of(&self, class: &str) -> bool {
        class.eq_ignore_ascii_case(ASSOCIATION_CACHE_ENTRY)
            || class.eq_ignore_ascii_case("Doctrine\\ORM\\Cache\\CacheEntry")
    }
}

/// `Doctrine\ORM\Cache\QueryCacheEntry`: a query's result (a list of
/// entity identifiers and associations) and its creation time.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryCacheEntry {
    pub result: PhpArray,
    /// Time creation of this cache entry (`microtime(true)`).
    pub time: f64,
}

impl QueryCacheEntry {
    /// `new QueryCacheEntry($result, $time)`: a time of `null` or `0`
    /// is now.
    pub fn new(result: PhpArray, time: Option<f64>) -> Self {
        let time = time
            .filter(|time| *time != 0.0)
            .unwrap_or_else(php_value::clock::microtime);
        Self { result, time }
    }
}

/// `Doctrine\ORM\Cache\TimestampCacheEntry`: the time a timestamped
/// space was updated.
#[derive(Debug, Clone, PartialEq)]
pub struct TimestampCacheEntry {
    pub time: f64,
}

impl TimestampCacheEntry {
    /// `new TimestampCacheEntry($time)`: a time of `null` is now.
    pub fn new(time: Option<f64>) -> Self {
        Self {
            time: time.unwrap_or_else(php_value::clock::microtime),
        }
    }
}

/// `Doctrine\ORM\Cache\CacheEntry`: an entry stored in a cache region.
#[derive(Debug, Clone, PartialEq)]
pub enum CacheEntry {
    Entity(EntityCacheEntry),
    Collection(CollectionCacheEntry),
    Association(AssociationCacheEntry),
    Query(QueryCacheEntry),
    Timestamp(TimestampCacheEntry),
    /// An entry of another class (such as the tests' `CacheEntryMock`,
    /// an `ArrayObject`): its class name and array.
    Other {
        class: String,
        data: PhpArray,
    },
}

impl CacheEntry {
    /// An entry of another `CacheEntry` class.
    pub fn other(class: impl Into<String>, data: PhpArray) -> Self {
        Self::Other {
            class: class.into(),
            data,
        }
    }
}

impl From<EntityCacheEntry> for CacheEntry {
    fn from(entry: EntityCacheEntry) -> Self {
        Self::Entity(entry)
    }
}

impl From<CollectionCacheEntry> for CacheEntry {
    fn from(entry: CollectionCacheEntry) -> Self {
        Self::Collection(entry)
    }
}

impl From<AssociationCacheEntry> for CacheEntry {
    fn from(entry: AssociationCacheEntry) -> Self {
        Self::Association(entry)
    }
}

impl From<QueryCacheEntry> for CacheEntry {
    fn from(entry: QueryCacheEntry) -> Self {
        Self::Query(entry)
    }
}

impl From<TimestampCacheEntry> for CacheEntry {
    fn from(entry: TimestampCacheEntry) -> Self {
        Self::Timestamp(entry)
    }
}
