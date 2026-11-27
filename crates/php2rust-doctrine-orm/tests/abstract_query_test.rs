//! Port of tests/Tests/ORM/AbstractQueryTest.php.
//!
//! The PHP test's `TestQuery extends AbstractQuery` is the
//! [`QueryState`] that every query type holds (`getResultCache()` reads
//! its profile's pool), over an entity manager with the test's
//! configuration (a stub there). The stub pool is an `ArrayAdapter`,
//! compared by identity.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/AbstractQueryTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php2rust_doctrine_dbal::cache::{ArrayAdapter, CacheItemPool, QueryCacheProfile};
use php2rust_doctrine_orm::abstract_query::QueryState;
use php2rust_doctrine_orm::configuration::Configuration;

fn stub() -> Arc<dyn CacheItemPool> {
    Arc::new(ArrayAdapter::new())
}

/// `$query->getResultCache()` of the `TestQuery`.
fn get_result_cache(query: &QueryState) -> Option<Arc<dyn CacheItemPool>> {
    query
        .get_query_cache_profile()
        .and_then(|profile| profile.get_result_cache().cloned())
}

fn same(a: &Arc<dyn CacheItemPool>, b: Option<&Arc<dyn CacheItemPool>>) -> bool {
    b.is_some_and(|b| Arc::ptr_eq(a, b))
}

#[test]
fn testItMakesHydrationCacheProfilesAwareOfTheResultCache() {
    let cache = stub();

    let entity_manager =
        orm::entity_manager_with(orm::connection(), |configuration: &mut Configuration| {
            configuration.set_hydration_cache(cache.clone());
        });
    let query = QueryState::new(&entity_manager);
    let cache_profile = QueryCacheProfile::default();

    query.set_hydration_cache_profile(Some(cache_profile));
    assert!(same(
        &cache,
        query
            .get_hydration_cache_profile()
            .unwrap()
            .get_result_cache()
    ));
}

#[test]
fn testItMakesResultCacheProfilesAwareOfTheResultCache() {
    let cache = stub();

    let entity_manager =
        orm::entity_manager_with(orm::connection(), |configuration: &mut Configuration| {
            configuration.set_result_cache(cache.clone());
        });
    let query = QueryState::new(&entity_manager);
    query.set_result_cache_profile(Some(QueryCacheProfile::default()));

    assert!(same(&cache, get_result_cache(&query).as_ref()));
}

#[test]
fn testSettingTheResultCacheIsPossibleWithoutCallingDeprecatedMethods() {
    let cache = stub();

    let entity_manager = orm::entity_manager_with(orm::connection(), |_: &mut Configuration| {});
    let query = QueryState::new(&entity_manager);
    query.set_result_cache(Some(cache.clone())).unwrap();

    assert!(same(&cache, get_result_cache(&query).as_ref()));
}
