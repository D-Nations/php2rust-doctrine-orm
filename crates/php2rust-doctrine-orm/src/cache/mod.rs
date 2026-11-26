//! `Doctrine\ORM\Cache`: the second-level cache.
//!
//! Upstream's `symfony/cache` pool is [`pool::ArrayAdapter`], a PSR-6
//! pool in memory.
//!
//! Upstream: doctrine/orm src/Cache.php

mod cache_entry;
mod cache_key;
pub mod configuration;
pub mod exception;
mod factory;
mod lock;
pub mod logging;
pub mod pool;
pub mod region;

pub use cache_entry::{
    ASSOCIATION_CACHE_ENTRY, AssociationCacheEntry, CacheEntry, CollectionCacheEntry,
    EntityCacheEntry, QueryCacheEntry, TimestampCacheEntry,
};
pub use cache_key::{
    CacheKey, CollectionCacheKey, EntityCacheKey, QueryCacheKey, TimestampCacheKey,
};
pub use configuration::{
    CacheConfiguration, QueryCacheValidator, RegionsConfiguration, TimestampQueryCacheValidator,
};
pub use factory::{CacheFactory, DefaultCacheFactory};
pub use lock::Lock;
pub use lock::uniqid;
pub use region::{
    ConcurrentRegion, DefaultRegion, FileLockRegion, Region, TimestampRegion, UpdateTimestampCache,
};

pub const DEFAULT_QUERY_REGION_NAME: &str = "query_cache_region";
pub const DEFAULT_TIMESTAMP_REGION_NAME: &str = "timestamp_cache_region";

/// May read items from the cache, but will not add items.
pub const MODE_GET: i64 = 1;
/// Will never read items from the cache, but will add items to the cache
/// as it reads them from the database.
pub const MODE_PUT: i64 = 2;
/// May read items from the cache, and add items to the cache.
pub const MODE_NORMAL: i64 = 3;
/// The query will never read items from the cache, but will refresh
/// items to the cache as it reads them from the database.
pub const MODE_REFRESH: i64 = 4;
