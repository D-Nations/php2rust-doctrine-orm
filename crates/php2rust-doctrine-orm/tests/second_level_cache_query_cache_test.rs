//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheQueryCacheTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use php_value::{Object, Thrown, Value};
use php2rust_doctrine_orm::cache::exception::EXCEPTION_CACHE_EXCEPTION;
use php2rust_doctrine_orm::cache::{
    CacheEntry, CacheKey, EntityCacheEntry, EntityCacheKey, MODE_GET, MODE_NORMAL, MODE_PUT,
    MODE_REFRESH, QueryCacheEntry, QueryCacheKey,
};
use php2rust_doctrine_orm::query::context::Hint;
use php2rust_doctrine_orm::query::dql_query::{HINT_CACHE_EVICT, HINT_FORCE_PARTIAL_LOAD};
use php2rust_doctrine_orm::query::{Query, ResultSetMapping};

use slc_functional::{
    ATTRACTION, CITY, COUNTRY, Fixture, STATE, assert_instance_of, count, entities, get_at,
    get_one, id, name, set_up,
};

const COUNTRY_DQL: &str = "SELECT c FROM Doctrine\\Tests\\Models\\Cache\\Country c";

/// `$this->_em->createQuery($dql)->setCacheable(true)->getResult()`.
async fn cacheable_result(f: &Fixture, dql: &str) -> Vec<Object> {
    let query = f.em.create_query(dql);
    query.set_cacheable(true);
    entities(&query.get_object_result().await.unwrap())
}

#[tokio::test]
async fn testBasicQueryCache() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    f.reset_query_log();
    let result1 = cacheable_result(&f, COUNTRY_DQL).await;

    assert_eq!(2, result1.len());
    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&result1[0]));
    assert_eq!(id(&f.countries[1]), id(&result1[1]));
    assert_eq!(name(&f.countries[0]), name(&result1[0]));
    assert_eq!(name(&f.countries[1]), name(&result1[1]));

    let query_region = f.get_default_query_region_name();
    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, COUNTRY_DQL).await;

    f.assert_query_count(1);
    assert_eq!(2, result2.len());

    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_hit_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));

    assert_instance_of(COUNTRY, &result2[0]);
    assert_instance_of(COUNTRY, &result2[1]);

    assert_eq!(id(&result1[0]), id(&result2[0]));
    assert_eq!(id(&result1[1]), id(&result2[1]));

    assert_eq!(name(&result1[0]), name(&result2[0]));
    assert_eq!(name(&result1[1]), name(&result2[1]));

    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_hit_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));
}

#[tokio::test]
async fn testQueryCacheModeGet() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;
    f.evict_regions();

    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    f.reset_query_log();
    let query_get = f.em.create_query(COUNTRY_DQL);
    query_get.set_cache_mode(Some(MODE_GET)).set_cacheable(true);

    // MODE_GET should never add items to the cache.
    assert_eq!(
        2,
        entities(&query_get.get_object_result().await.unwrap()).len()
    );
    f.assert_query_count(1);

    assert_eq!(
        2,
        entities(&query_get.get_object_result().await.unwrap()).len()
    );
    f.assert_query_count(2);

    let result = cacheable_result(&f, COUNTRY_DQL).await;

    assert_eq!(2, result.len());
    f.assert_query_count(3);

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    // MODE_GET should read items if exists.
    assert_eq!(
        2,
        entities(&query_get.get_object_result().await.unwrap()).len()
    );
    f.assert_query_count(3);
}

#[tokio::test]
async fn testQueryCacheModePut() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;
    f.evict_regions();

    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    f.reset_query_log();
    let result = cacheable_result(&f, COUNTRY_DQL).await;

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    assert_eq!(2, result.len());
    f.assert_query_count(1);

    let query_put = f.em.create_query(COUNTRY_DQL);
    query_put.set_cache_mode(Some(MODE_PUT)).set_cacheable(true);

    // MODE_PUT should never read itens from cache.
    assert_eq!(
        2,
        entities(&query_put.get_object_result().await.unwrap()).len()
    );
    f.assert_query_count(2);
    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    assert_eq!(
        2,
        entities(&query_put.get_object_result().await.unwrap()).len()
    );
    f.assert_query_count(3);
    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));
}

#[tokio::test]
async fn testQueryCacheModeRefresh() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;
    f.evict_regions();

    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));

    let region = f.cache.get_entity_cache_region(COUNTRY).unwrap().unwrap();
    f.reset_query_log();
    let result = cacheable_result(&f, COUNTRY_DQL).await;

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    assert_eq!(2, result.len());
    f.assert_query_count(1);

    let country_id1 = id(&f.countries[0]);
    let country_id2 = id(&f.countries[1]);
    let country_name1 = name(&f.countries[0]);
    let country_name2 = name(&f.countries[1]);

    let key1 = EntityCacheKey::new(COUNTRY, &slc::array(&[("id", country_id1.clone())]));
    let key2 = EntityCacheKey::new(COUNTRY, &slc::array(&[("id", country_id2.clone())]));
    let entry1 = EntityCacheEntry::new(
        COUNTRY,
        slc::array(&[("id", country_id1), ("name", Value::string("outdated"))]),
    );
    let entry2 = EntityCacheEntry::new(
        COUNTRY,
        slc::array(&[("id", country_id2), ("name", Value::string("outdated"))]),
    );

    region.put(&key1.into(), entry1.into(), None).unwrap();
    region.put(&key2.into(), entry2.into(), None).unwrap();
    f.em.clear().unwrap();

    let query_refresh = f.em.create_query(COUNTRY_DQL);
    query_refresh
        .set_cache_mode(Some(MODE_REFRESH))
        .set_cacheable(true);

    // MODE_REFRESH should never read itens from cache.
    let result1 = entities(&query_refresh.get_object_result().await.unwrap());
    assert_eq!(2, result1.len());
    assert_eq!(country_name1, name(&result1[0]));
    assert_eq!(country_name2, name(&result1[1]));
    f.assert_query_count(2);

    f.em.clear().unwrap();

    let result2 = entities(&query_refresh.get_object_result().await.unwrap());
    assert_eq!(2, result2.len());
    assert_eq!(country_name1, name(&result2[0]));
    assert_eq!(country_name2, name(&result2[1]));
    f.assert_query_count(3);
}

#[tokio::test]
async fn testBasicQueryCachePutEntityCache() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.evict_regions();
    f.logger.clear_stats();
    f.em.clear().unwrap();

    f.reset_query_log();
    let result1 = cacheable_result(&f, COUNTRY_DQL).await;

    assert_eq!(2, result1.len());
    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&result1[0]));
    assert_eq!(id(&f.countries[1]), id(&result1[1]));
    assert_eq!(name(&f.countries[0]), name(&result1[0]));
    assert_eq!(name(&f.countries[1]), name(&result1[1]));

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    let query_region = f.get_default_query_region_name();
    assert_eq!(3, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(COUNTRY))
    );

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, COUNTRY_DQL).await;

    f.assert_query_count(1);
    assert_eq!(2, result2.len());

    assert_eq!(3, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_hit_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));

    assert_instance_of(COUNTRY, &result2[0]);
    assert_instance_of(COUNTRY, &result2[1]);

    assert_eq!(id(&result1[0]), id(&result2[0]));
    assert_eq!(id(&result1[1]), id(&result2[1]));

    assert_eq!(name(&result1[0]), name(&result2[0]));
    assert_eq!(name(&result1[1]), name(&result2[1]));

    assert_eq!(3, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_hit_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));
}

/// The assertions on the states, their cities and the cities'
/// attractions of `testMultipleNestedDQLAliases()`.
async fn assert_nested(result: &[Object]) {
    assert_instance_of(STATE, &result[0]);
    assert_instance_of(STATE, &result[1]);

    assert_eq!(2, count(&result[0], "cities").await);
    assert_eq!(2, count(&result[1], "cities").await);

    for state in &result[..2] {
        for index in 0..2 {
            assert_instance_of(CITY, &get_at(state, "cities", index).await);
        }
    }

    assert_eq!(
        2,
        count(&get_at(&result[0], "cities", 0).await, "attractions").await
    );
    assert_eq!(
        2,
        count(&get_at(&result[0], "cities", 1).await, "attractions").await
    );
    assert_eq!(
        2,
        count(&get_at(&result[1], "cities", 0).await, "attractions").await
    );
    assert_eq!(
        1,
        count(&get_at(&result[1], "cities", 1).await, "attractions").await
    );
}

#[tokio::test]
async fn testMultipleNestedDQLAliases() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_attractions().await;

    let _query_region_name = f.get_default_query_region_name();
    let _city_region_name = f.get_entity_region(CITY);
    let _state_region_name = f.get_entity_region(STATE);
    let _attraction_region_name = f.get_entity_region(ATTRACTION);

    f.logger.clear_stats();
    f.evict_regions();
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql = "SELECT s, c, a FROM Doctrine\\Tests\\Models\\Cache\\State s JOIN s.cities c JOIN c.attractions a";
    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(2, result1.len());
    f.assert_query_count(1);

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));

    for city in &f.cities[..4] {
        assert!(f.contains_entity(CITY, city));
    }

    for attraction in &f.attractions[..4] {
        assert!(f.contains_entity(ATTRACTION, attraction));
    }

    assert_nested(&result1).await;

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(2, result2.len());
    f.assert_query_count(1);

    assert_nested(&result2).await;
}

#[tokio::test]
async fn testBasicQueryParams() {
    let mut f = set_up().await;
    f.evict_regions();

    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    f.reset_query_log();
    let country_name = name(&f.countries[0]);
    let dql = "SELECT c FROM Doctrine\\Tests\\Models\\Cache\\Country c WHERE c.name = :name";
    let query = f.em.create_query(dql);
    query
        .set_cacheable(true)
        .set_parameter("name", country_name.clone(), None);
    let result1 = entities(&query.get_object_result().await.unwrap());

    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&result1[0]));
    assert_eq!(name(&f.countries[0]), name(&result1[0]));

    f.em.clear().unwrap();

    let query = f.em.create_query(dql);
    query
        .set_cacheable(true)
        .set_parameter("name", country_name, None);
    let result2 = entities(&query.get_object_result().await.unwrap());

    f.assert_query_count(1);
    assert_eq!(1, result2.len());

    assert_instance_of(COUNTRY, &result2[0]);

    assert_eq!(id(&result1[0]), id(&result2[0]));
    assert_eq!(name(&result1[0]), name(&result2[0]));
}

#[tokio::test]
async fn testLoadFromDatabaseWhenEntityMissing() {
    let mut f = set_up().await;
    f.evict_regions();

    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    f.reset_query_log();
    let result1 = cacheable_result(&f, COUNTRY_DQL).await;

    assert_eq!(2, result1.len());
    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&result1[0]));
    assert_eq!(id(&f.countries[1]), id(&result1[1]));
    assert_eq!(name(&f.countries[0]), name(&result1[0]));
    assert_eq!(name(&f.countries[1]), name(&result1[1]));

    let query_region = f.get_default_query_region_name();
    assert_eq!(3, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));

    f.cache.evict_entity(COUNTRY, &id(&result1[0])).unwrap();
    assert!(!f.contains_entity(COUNTRY, &result1[0]));

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, COUNTRY_DQL).await;

    f.assert_query_count(2);
    assert_eq!(2, result2.len());

    assert_eq!(5, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_miss_count());
    assert_eq!(2, f.logger.get_region_put_count(&query_region));
    assert_eq!(2, f.logger.get_region_miss_count(&query_region));

    assert_instance_of(COUNTRY, &result2[0]);
    assert_instance_of(COUNTRY, &result2[1]);

    assert_eq!(id(&result1[0]), id(&result2[0]));
    assert_eq!(id(&result1[1]), id(&result2[1]));

    assert_eq!(name(&result1[0]), name(&result2[0]));
    assert_eq!(name(&result1[1]), name(&result2[1]));

    f.assert_query_count(2);
}

/// The assertions on the states and their cities of
/// `testBasicQueryFetchJoinsOneToMany()`.
async fn assert_states_with_cities(result: &[Object], loaded: bool) {
    assert_instance_of(STATE, &result[0]);
    assert_instance_of(STATE, &result[1]);
    assert_eq!(2, count(&result[0], "cities").await);
    assert_eq!(2, count(&result[1], "cities").await);

    for state in &result[..2] {
        for index in 0..2 {
            let city = get_at(state, "cities", index).await;
            assert_instance_of(CITY, &city);
            if loaded {
                assert!(!id(&city).is_null());
                assert!(!name(&city).is_null());
            }
        }
    }
}

#[tokio::test]
async fn testBasicQueryFetchJoinsOneToMany() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.evict_regions();
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql = "SELECT s, c FROM Doctrine\\Tests\\Models\\Cache\\State s JOIN s.cities c";
    let result1 = cacheable_result(&f, dql).await;

    f.assert_query_count(1);
    assert_states_with_cities(&result1, true).await;

    f.em.clear().unwrap();

    let result2 = cacheable_result(&f, dql).await;

    assert_states_with_cities(&result2, true).await;

    f.assert_query_count(1);
}

#[tokio::test]
async fn testBasicQueryFetchJoinsManyToOne() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.em.clear().unwrap();

    f.evict_regions();
    f.logger.clear_stats();

    f.reset_query_log();
    let dql = "SELECT c, s FROM Doctrine\\Tests\\Models\\Cache\\City c JOIN c.state s";
    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(4, result1.len());
    assert_instance_of(CITY, &result1[0]);
    assert_instance_of(CITY, &result1[1]);
    assert_instance_of(STATE, &get_one(&result1[0], "state"));
    assert_instance_of(STATE, &get_one(&result1[1], "state"));

    assert!(f.contains_entity(CITY, &result1[0]));
    assert!(f.contains_entity(CITY, &result1[1]));
    assert!(f.contains_entity(STATE, &get_one(&result1[0], "state")));
    assert!(f.contains_entity(STATE, &get_one(&result1[1], "state")));

    let query_region = f.get_default_query_region_name();
    assert_eq!(7, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));
    assert_eq!(
        2,
        f.logger.get_region_put_count(&f.get_entity_region(STATE))
    );
    assert_eq!(4, f.logger.get_region_put_count(&f.get_entity_region(CITY)));

    f.assert_query_count(1);

    f.em.clear().unwrap();
    f.logger.clear_stats();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(4, result1.len());
    assert_instance_of(CITY, &result2[0]);
    assert_instance_of(CITY, &result2[1]);
    assert_instance_of(STATE, &get_one(&result2[0], "state"));
    assert_instance_of(STATE, &get_one(&result2[1], "state"));

    assert!(!id(&result2[0]).is_null());
    assert!(!id(&get_one(&result2[1], "state")).is_null());

    assert!(!name(&result2[0]).is_null());
    assert!(!name(&get_one(&result2[1], "state")).is_null());

    assert_eq!(name(&result1[0]), name(&result2[0]));
    assert_eq!(name(&result1[1]), name(&result2[1]));
    assert_eq!(
        name(&get_one(&result1[0], "state")),
        name(&get_one(&result2[0], "state"))
    );
    assert_eq!(
        name(&get_one(&result1[1], "state")),
        name(&get_one(&result2[1], "state"))
    );

    f.assert_query_count(1);
}

#[tokio::test]
async fn testReloadQueryIfToOneIsNotFound() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.em.clear().unwrap();

    f.evict_regions();
    f.logger.clear_stats();

    f.reset_query_log();
    let dql = "SELECT c, s FROM Doctrine\\Tests\\Models\\Cache\\City c JOIN c.state s";
    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(4, result1.len());
    assert_instance_of(CITY, &result1[0]);
    assert_instance_of(CITY, &result1[1]);
    assert_instance_of(STATE, &get_one(&result1[0], "state"));
    assert_instance_of(STATE, &get_one(&result1[1], "state"));

    assert!(f.contains_entity(CITY, &result1[0]));
    assert!(f.contains_entity(CITY, &result1[1]));
    assert!(f.contains_entity(STATE, &get_one(&result1[0], "state")));
    assert!(f.contains_entity(STATE, &get_one(&result1[1], "state")));
    f.assert_query_count(1);

    f.em.clear().unwrap();

    f.cache.evict_entity_region(STATE).unwrap();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(4, result1.len());
    assert_instance_of(CITY, &result2[0]);
    assert_instance_of(CITY, &result2[1]);
    assert_instance_of(STATE, &get_one(&result2[0], "state"));
    assert_instance_of(STATE, &get_one(&result2[1], "state"));

    f.assert_query_count(2);
}

#[tokio::test]
async fn testReloadQueryIfToManyAssociationItemIsNotFound() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.evict_regions();
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql = "SELECT s, c FROM Doctrine\\Tests\\Models\\Cache\\State s JOIN s.cities c";
    let result1 = cacheable_result(&f, dql).await;

    f.assert_query_count(1);
    assert_states_with_cities(&result1, false).await;

    f.em.clear().unwrap();

    f.cache.evict_entity_region(CITY).unwrap();

    let result2 = cacheable_result(&f, dql).await;

    assert_states_with_cities(&result2, false).await;

    f.assert_query_count(2);
}

#[tokio::test]
async fn testBasicNativeQueryCache() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.logger.clear_stats();
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(COUNTRY, "c", None);
    rsm.add_field_result("c", "name", "name", None);
    rsm.add_field_result("c", "id", "id", None);

    f.reset_query_log();
    let sql = "SELECT id, name FROM cache_country";
    let query = f.em.create_native_query(sql, rsm.clone()).unwrap();
    query.set_cacheable(true);
    let result1 = entities(&query.get_object_result().await.unwrap());

    assert_eq!(2, result1.len());
    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&result1[0]));
    assert_eq!(id(&f.countries[1]), id(&result1[1]));
    assert_eq!(name(&f.countries[0]), name(&result1[0]));
    assert_eq!(name(&f.countries[1]), name(&result1[1]));

    let query_region = f.get_default_query_region_name();
    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));

    f.em.clear().unwrap();

    let query = f.em.create_native_query(sql, rsm).unwrap();
    query.set_cacheable(true);
    let result2 = entities(&query.get_object_result().await.unwrap());

    f.assert_query_count(1);
    assert_eq!(2, result2.len());

    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_hit_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));

    assert_instance_of(COUNTRY, &result2[0]);
    assert_instance_of(COUNTRY, &result2[1]);

    assert_eq!(id(&result1[0]), id(&result2[0]));
    assert_eq!(id(&result1[1]), id(&result2[1]));

    assert_eq!(name(&result1[0]), name(&result2[0]));
    assert_eq!(name(&result1[1]), name(&result2[1]));

    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_miss_count());

    assert_eq!(1, f.logger.get_region_put_count(&query_region));
    assert_eq!(1, f.logger.get_region_hit_count(&query_region));
    assert_eq!(1, f.logger.get_region_miss_count(&query_region));
}

#[tokio::test]
async fn testQueryDependsOnFirstAndMaxResultResult() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.logger.clear_stats();
    f.em.clear().unwrap();

    f.reset_query_log();
    let query = f.em.create_query(COUNTRY_DQL);
    query
        .set_cacheable(true)
        .set_first_result(1)
        .set_max_results(Some(1));
    let _result1 = query.get_object_result().await.unwrap();

    f.assert_query_count(1);
    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());

    f.em.clear().unwrap();

    let query = f.em.create_query(COUNTRY_DQL);
    query
        .set_cacheable(true)
        .set_first_result(2)
        .set_max_results(Some(1));
    let _result2 = query.get_object_result().await.unwrap();

    f.assert_query_count(2);
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());

    f.em.clear().unwrap();

    let _result3 = cacheable_result(&f, COUNTRY_DQL).await;

    f.assert_query_count(3);
    assert_eq!(3, f.logger.get_put_count());
    assert_eq!(3, f.logger.get_miss_count());
}

#[tokio::test]
async fn testQueryCacheLifetime() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.logger.clear_stats();
    f.em.clear().unwrap();

    f.reset_query_log();
    let query = f.em.create_query(COUNTRY_DQL);
    query.set_cacheable(true).set_lifetime(3600);
    let result1 = entities(&query.get_object_result().await.unwrap());

    assert!(!result1.is_empty());
    f.assert_query_count(1);
    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());

    f.em.clear().unwrap();

    let key: CacheKey =
        QueryCacheKey::with(query.get_hash().unwrap(), 3600, MODE_NORMAL, None).into();
    let region = f.cache.get_query_cache(None).unwrap().get_region();
    let entry = region.get(&key).unwrap();

    let Some(CacheEntry::Query(entry)) = entry else {
        panic!("not a QueryCacheEntry: {entry:?}");
    };
    let entry = QueryCacheEntry::new(entry.result, Some(entry.time / 2.0));

    region.put(&key, entry.into(), None).unwrap();

    let query = f.em.create_query(COUNTRY_DQL);
    query.set_cacheable(true).set_lifetime(3600);
    let result2 = entities(&query.get_object_result().await.unwrap());

    assert!(!result2.is_empty());
    f.assert_query_count(2);
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
}

#[tokio::test]
async fn testQueryCacheRegion() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    f.logger.clear_stats();
    f.em.clear().unwrap();

    f.reset_query_log();
    let query = f.em.create_query(COUNTRY_DQL);

    let query1 = query.clone_query();
    query1.set_cacheable(true).set_cache_region("foo_region");
    let result1 = entities(&query1.get_object_result().await.unwrap());

    assert!(!result1.is_empty());
    f.assert_query_count(1);
    assert_eq!(0, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count("foo_region"));
    assert_eq!(1, f.logger.get_region_miss_count("foo_region"));

    let query2 = query.clone_query();
    query2.set_cacheable(true).set_cache_region("bar_region");
    let result2 = entities(&query2.get_object_result().await.unwrap());

    assert!(!result2.is_empty());
    f.assert_query_count(2);
    assert_eq!(0, f.logger.get_hit_count());
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count("bar_region"));
    assert_eq!(1, f.logger.get_region_miss_count("bar_region"));

    let query3 = query.clone_query();
    query3.set_cacheable(true).set_cache_region("foo_region");
    let result3 = entities(&query3.get_object_result().await.unwrap());

    assert!(!result3.is_empty());
    f.assert_query_count(2);
    assert_eq!(3, f.logger.get_hit_count());
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_hit_count("foo_region"));
    assert_eq!(1, f.logger.get_region_put_count("foo_region"));
    assert_eq!(1, f.logger.get_region_miss_count("foo_region"));

    let query4 = query.clone_query();
    query4.set_cacheable(true).set_cache_region("bar_region");
    let _result4 = entities(&query4.get_object_result().await.unwrap());

    assert!(!result3.is_empty());
    f.assert_query_count(2);
    assert_eq!(6, f.logger.get_hit_count());
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_hit_count("bar_region"));
    assert_eq!(1, f.logger.get_region_put_count("bar_region"));
    assert_eq!(1, f.logger.get_region_miss_count("bar_region"));
}

/// `(clone $query)->setParameter('id', $id)->setCacheable(true)
/// ->setMaxResults(1)->getSingleResult()`.
async fn single_by_id(query: &Query, id: Value) -> Object {
    let query = query.clone_query();
    query
        .set_parameter("id", id, None)
        .set_cacheable(true)
        .set_max_results(Some(1));
    orm::obj(&query.get_single_result(None).await.unwrap())
}

#[tokio::test]
async fn testResolveAssociationCacheEntry() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;

    f.em.clear().unwrap();

    let state_id = id(&f.states[0]);
    let country_name = name(&get_one(&f.states[0], "country"));
    let dql = "SELECT s FROM Doctrine\\Tests\\Models\\Cache\\State s WHERE s.id = :id";
    let query = f.em.create_query(dql);
    f.reset_query_log();

    let state1 = single_by_id(&query, state_id.clone()).await;

    let country1 = get_one(&state1, "country");
    f.assert_query_count(1);
    assert_instance_of(STATE, &state1);
    f.em.initialize_object(&country1).await.unwrap();
    assert_eq!(country_name, name(&country1));
    assert_eq!(state_id, id(&state1));

    f.em.clear().unwrap();

    f.reset_query_log();
    let state2 = single_by_id(&query, state_id.clone()).await;

    let country2 = get_one(&state2, "country");
    f.assert_query_count(0);
    assert_instance_of(STATE, &state2);
    f.em.initialize_object(&country2).await.unwrap();
    assert_eq!(country_name, name(&country2));
    assert_eq!(state_id, id(&state2));
}

#[tokio::test]
async fn testResolveToOneAssociationCacheEntry() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.evict_regions();

    f.em.clear().unwrap();

    let city_id = id(&f.cities[0]);
    let dql =
        "SELECT c, s FROM Doctrine\\Tests\\Models\\Cache\\City c JOIN c.state s WHERE c.id = :id";
    let query = f.em.create_query(dql);
    f.reset_query_log();

    let city1 = single_by_id(&query, city_id.clone()).await;

    f.assert_query_count(1);
    assert_instance_of(CITY, &city1);
    assert_instance_of(STATE, &get_one(&city1, "state"));
    let city = get_at(&get_one(&city1, "state"), "cities", 0).await;
    assert_instance_of(CITY, &city);
    assert_instance_of(STATE, &get_one(&city, "state"));

    f.em.clear().unwrap();

    f.reset_query_log();
    let city2 = single_by_id(&query, city_id).await;

    f.assert_query_count(0);
    assert_instance_of(CITY, &city2);
    assert_instance_of(STATE, &get_one(&city2, "state"));
    let city = get_at(&get_one(&city2, "state"), "cities", 0).await;
    assert_instance_of(CITY, &city);
    assert_instance_of(STATE, &get_one(&city, "state"));
}

#[tokio::test]
async fn testResolveToManyAssociationCacheEntry() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.evict_regions();

    f.em.clear().unwrap();

    let state_id = id(&f.states[0]);
    let dql =
        "SELECT s, c FROM Doctrine\\Tests\\Models\\Cache\\State s JOIN s.cities c WHERE s.id = :id";
    let query = f.em.create_query(dql);
    f.reset_query_log();

    let state1 = single_by_id(&query, state_id.clone()).await;

    f.assert_query_count(1);
    assert_instance_of(STATE, &state1);
    let city = get_at(&state1, "cities", 0).await;
    assert_instance_of(CITY, &city);
    assert_instance_of(STATE, &get_one(&city, "state"));
    assert!(orm::same(&state1, &get_one(&city, "state")));

    f.em.clear().unwrap();

    f.reset_query_log();
    let state2 = single_by_id(&query, state_id).await;

    f.assert_query_count(0);
    assert_instance_of(STATE, &state2);
    let city = get_at(&state2, "cities", 0).await;
    assert_instance_of(CITY, &city);
    assert_instance_of(STATE, &get_one(&city, "state"));
    assert!(orm::same(&state2, &get_one(&city, "state")));
}

#[tokio::test]
async fn testHintClearEntityRegionUpdateStatement() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    let query =
        f.em.create_query("DELETE Doctrine\\Tests\\Models\\Cache\\Country u WHERE u.id = 4");
    query.set_hint(HINT_CACHE_EVICT, Hint::Value(Value::Bool(true)));
    query.execute(None, None).await.unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));
}

#[tokio::test]
async fn testHintClearEntityRegionDeleteStatement() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(f.contains_entity(COUNTRY, &f.countries[1]));

    let query = f.em.create_query(
        "UPDATE Doctrine\\Tests\\Models\\Cache\\Country u SET u.name = 'foo' WHERE u.id = 1",
    );
    query.set_hint(HINT_CACHE_EVICT, Hint::Value(Value::Bool(true)));
    query.execute(None, None).await.unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));
    assert!(!f.contains_entity(COUNTRY, &f.countries[1]));
}

/// `expectException(CacheException::class)` and
/// `expectExceptionMessage($message)`.
#[track_caller]
fn assert_cache_exception(result: Result<Value, Thrown>, message: &str) {
    let e = result.expect_err("expected a CacheException");
    assert!(
        e.class == EXCEPTION_CACHE_EXCEPTION
            || php2rust_doctrine_persistence::php_reflection::is_a(
                &e.class,
                EXCEPTION_CACHE_EXCEPTION
            ),
        "{} is not a CacheException",
        e.class
    );
    assert!(orm::message(&e).contains(message), "{}", orm::message(&e));
}

#[tokio::test]
async fn testCacheablePartialQueryException() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    let query =
        f.em.create_query("SELECT PARTIAL c.{id} FROM Doctrine\\Tests\\Models\\Cache\\Country c");
    query.set_cacheable(true);
    assert_cache_exception(
        query.get_object_result().await,
        "Second level cache does not support partial entities.",
    );
}

#[tokio::test]
async fn testCacheableForcePartialLoadHintQueryException() {
    let mut f = set_up().await;
    f.evict_regions();
    f.load_fixtures_countries().await;

    let query = f.em.create_query(COUNTRY_DQL);
    query
        .set_cacheable(true)
        .set_hint(HINT_FORCE_PARTIAL_LOAD, Hint::Value(Value::Bool(true)));
    assert_cache_exception(
        query.get_object_result().await,
        "Second level cache does not support partial entities.",
    );
}

#[tokio::test]
async fn testNonCacheableQueryDeleteStatementException() {
    let f = set_up().await;
    let query =
        f.em.create_query("DELETE Doctrine\\Tests\\Models\\Cache\\Country u WHERE u.id = 4");
    query.set_cacheable(true);
    assert_cache_exception(
        query.get_object_result().await,
        "Second-level cache query supports only select statements.",
    );
}

#[tokio::test]
async fn testNonCacheableQueryUpdateStatementException() {
    let f = set_up().await;
    let query = f.em.create_query(
        "UPDATE Doctrine\\Tests\\Models\\Cache\\Country u SET u.name = 'foo' WHERE u.id = 4",
    );
    query.set_cacheable(true);
    assert_cache_exception(
        query.get_object_result().await,
        "Second-level cache query supports only select statements.",
    );
}

#[tokio::test]
async fn testQueryCacheShouldBeEvictedOnTimestampUpdate() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    f.reset_query_log();
    let dql = "SELECT country FROM Doctrine\\Tests\\Models\\Cache\\Country country";

    let result1 = cacheable_result(&f, dql).await;

    assert_eq!(2, result1.len());
    f.assert_query_count(1);

    f.em.persist(&slc::country("France")).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset_query_log();

    let result2 = cacheable_result(&f, dql).await;

    assert_eq!(3, result2.len());
    f.assert_query_count(1);

    for entity in &result2 {
        assert_instance_of(COUNTRY, entity);
    }
}
