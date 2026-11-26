//! The keys of the second-level cache regions: `EntityCacheKey`,
//! `CollectionCacheKey`, `QueryCacheKey` and `TimestampCacheKey`, and a
//! key of another class (`CacheKey` is abstract upstream; a subclass
//! only names its hash).
//!
//! Upstream: doctrine/orm src/Cache/CacheKey.php src/Cache/EntityCacheKey.php
//! Upstream: doctrine/orm src/Cache/CollectionCacheKey.php src/Cache/QueryCacheKey.php
//! Upstream: doctrine/orm src/Cache/TimestampCacheKey.php

use php_value::PhpArray;

use crate::utility::implode_values;

use super::MODE_NORMAL;

/// `str_replace('\\', '.', strtolower($class))`.
fn class_hash(class: &str) -> String {
    class.to_ascii_lowercase().replace('\\', ".")
}

/// `Doctrine\ORM\Cache\EntityCacheKey`: an entity class's role in a
/// region. In an inheritance hierarchy the class is the root entity.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityCacheKey {
    pub hash: String,
    pub entity_class: String,
    /// The entity identifier, sorted by field name (`ksort()`).
    pub identifier: PhpArray,
}

impl EntityCacheKey {
    pub fn new(entity_class: impl Into<String>, identifier: &PhpArray) -> Self {
        let entity_class = entity_class.into();
        let mut identifier = identifier.clone();
        identifier.ksort();
        let hash = format!(
            "{}_{}",
            entity_class.to_ascii_lowercase(),
            implode_values(&identifier)
        )
        .replace('\\', ".");
        Self {
            hash,
            entity_class,
            identifier,
        }
    }
}

/// `Doctrine\ORM\Cache\CollectionCacheKey`: an entity collection's role
/// in a region.
#[derive(Debug, Clone, PartialEq)]
pub struct CollectionCacheKey {
    pub hash: String,
    /// The owner entity class.
    pub entity_class: String,
    pub association: String,
    /// The identifier of the owning entity, sorted by field name.
    pub owner_identifier: PhpArray,
}

impl CollectionCacheKey {
    pub fn new(
        entity_class: impl Into<String>,
        association: impl Into<String>,
        owner_identifier: &PhpArray,
    ) -> Self {
        Self::with_filter_hash(entity_class, association, owner_identifier, "")
    }

    /// The key of a collection whose SQL filters hash to `filter_hash`.
    pub fn with_filter_hash(
        entity_class: impl Into<String>,
        association: impl Into<String>,
        owner_identifier: &PhpArray,
        filter_hash: &str,
    ) -> Self {
        let entity_class = entity_class.into();
        let association = association.into();
        let mut owner_identifier = owner_identifier.clone();
        owner_identifier.ksort();
        let filter_hash = if filter_hash.is_empty() {
            String::new()
        } else {
            format!("_{filter_hash}")
        };
        let hash = format!(
            "{}_{}__{association}{filter_hash}",
            class_hash(&entity_class),
            implode_values(&owner_identifier)
        );
        Self {
            hash,
            entity_class,
            association,
            owner_identifier,
        }
    }
}

/// `Doctrine\ORM\Cache\QueryCacheKey`: a key that identifies a
/// particular query.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryCacheKey {
    pub hash: String,
    pub lifetime: i64,
    /// One of the `Cache::MODE_*` constants.
    pub cache_mode: i64,
    pub timestamp_key: Option<TimestampCacheKey>,
}

impl QueryCacheKey {
    /// `new QueryCacheKey($cacheId)`: no lifetime, `Cache::MODE_NORMAL`,
    /// no timestamp key.
    pub fn new(cache_id: impl Into<String>) -> Self {
        Self::with(cache_id, 0, MODE_NORMAL, None)
    }

    pub fn with(
        cache_id: impl Into<String>,
        lifetime: i64,
        cache_mode: i64,
        timestamp_key: Option<TimestampCacheKey>,
    ) -> Self {
        Self {
            hash: cache_id.into(),
            lifetime,
            cache_mode,
            timestamp_key,
        }
    }
}

/// `Doctrine\ORM\Cache\TimestampCacheKey`: a key that identifies a
/// timestamped space.
#[derive(Debug, Clone, PartialEq)]
pub struct TimestampCacheKey {
    pub hash: String,
}

impl TimestampCacheKey {
    pub fn new(space: impl Into<String>) -> Self {
        Self { hash: space.into() }
    }
}

/// `Doctrine\ORM\Cache\CacheKey`: a key stored in a cache region.
#[derive(Debug, Clone, PartialEq)]
pub enum CacheKey {
    Entity(EntityCacheKey),
    Collection(CollectionCacheKey),
    Query(QueryCacheKey),
    Timestamp(TimestampCacheKey),
    /// A key of another subclass (such as the tests' `CacheKeyMock`):
    /// its class name and hash.
    Other {
        class: String,
        hash: String,
    },
}

impl CacheKey {
    /// A key of another `CacheKey` subclass.
    pub fn other(class: impl Into<String>, hash: impl Into<String>) -> Self {
        Self::Other {
            class: class.into(),
            hash: hash.into(),
        }
    }

    /// `$key->hash`.
    pub fn hash(&self) -> &str {
        match self {
            Self::Entity(key) => &key.hash,
            Self::Collection(key) => &key.hash,
            Self::Query(key) => &key.hash,
            Self::Timestamp(key) => &key.hash,
            Self::Other { hash, .. } => hash,
        }
    }

    /// The key's class name.
    pub fn class(&self) -> &str {
        match self {
            Self::Entity(_) => "Doctrine\\ORM\\Cache\\EntityCacheKey",
            Self::Collection(_) => "Doctrine\\ORM\\Cache\\CollectionCacheKey",
            Self::Query(_) => "Doctrine\\ORM\\Cache\\QueryCacheKey",
            Self::Timestamp(_) => "Doctrine\\ORM\\Cache\\TimestampCacheKey",
            Self::Other { class, .. } => class,
        }
    }
}

impl From<EntityCacheKey> for CacheKey {
    fn from(key: EntityCacheKey) -> Self {
        Self::Entity(key)
    }
}

impl From<CollectionCacheKey> for CacheKey {
    fn from(key: CollectionCacheKey) -> Self {
        Self::Collection(key)
    }
}

impl From<QueryCacheKey> for CacheKey {
    fn from(key: QueryCacheKey) -> Self {
        Self::Query(key)
    }
}

impl From<TimestampCacheKey> for CacheKey {
    fn from(key: TimestampCacheKey) -> Self {
        Self::Timestamp(key)
    }
}
