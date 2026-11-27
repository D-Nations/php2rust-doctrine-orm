//! The PSR-6 cache item pool where upstream's tests and
//! `DefaultCacheFactory` take a `Psr\Cache\CacheItemPoolInterface` (the
//! tests use `Symfony\Component\Cache\Adapter\ArrayAdapter`). The pool is
//! DBAL's (`php2rust_doctrine_dbal::cache::pool`), which its result cache
//! uses too; this module re-exports it.
//!
//! The ORM keeps its own objects in a pool as [`PoolValue::Object`]: a
//! second-level cache entry ([`CacheEntry`](super::CacheEntry)) or a
//! parsed DQL query ([`CachedParserResult`]), read back with
//! [`PoolValue::downcast_ref`].
//!
//! Upstream: none

use std::sync::Arc;

pub use php2rust_doctrine_dbal::cache::pool::{
    ArrayAdapter, CacheItem, CacheItemPool, CachedResult, PSR_INVALID_ARGUMENT_EXCEPTION,
    PoolObject, PoolValue, RESERVED_CHARACTERS, validate_key,
};

use crate::query::ParserResult;

/// A `ParserResult` in a pool. PHP stores a serialized copy; the parser
/// result is not changed once parsed, so the pool shares it. Two are equal
/// when they are the same result.
#[derive(Debug, Clone)]
pub struct CachedParserResult(pub Arc<ParserResult>);

impl PartialEq for CachedParserResult {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
