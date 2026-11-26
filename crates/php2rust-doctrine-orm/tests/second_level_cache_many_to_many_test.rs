//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheManyToManyTest.php

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

use php2rust_doctrine_orm::cache::exception::EXCEPTION_CACHE_EXCEPTION;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_persistence::php_reflection;

use slc_functional::{
    CITY, STATE, TRAVEL, TRAVELER, add_visited_city, assert_instance_of, count, get_at, id, name,
    set_up, travel, traveler,
};

/// The fixtures of the tests: countries, states, cities, travelers and
/// travels.
async fn load_travels(f: &mut slc_functional::Fixture) {
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_traveler().await;
    f.load_fixtures_travels().await;
}

#[tokio::test]
async fn testShouldPutManyToManyCollectionOwningSideOnPersist() {
    let mut f = set_up().await;
    f.evict_regions();

    load_travels(&mut f).await;
    f.em.clear().unwrap();

    assert!(f.contains_entity(TRAVEL, &f.travels[0]));
    assert!(f.contains_entity(TRAVEL, &f.travels[1]));

    assert!(f.contains_collection(TRAVEL, "visitedCities", &f.travels[0]));
    assert!(f.contains_collection(TRAVEL, "visitedCities", &f.travels[1]));

    for city in &f.cities[..4] {
        assert!(f.contains_entity(CITY, city));
    }
}

#[tokio::test]
async fn testPutAndLoadManyToManyRelation() {
    let mut f = set_up().await;
    f.evict_regions();

    load_travels(&mut f).await;

    f.em.clear().unwrap();
    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_entity_region(TRAVEL).unwrap();
    f.cache
        .evict_collection_region(TRAVEL, "visitedCities")
        .unwrap();

    f.logger.clear_stats();

    assert!(!f.contains_entity(TRAVEL, &f.travels[0]));
    assert!(!f.contains_entity(TRAVEL, &f.travels[1]));

    assert!(!f.contains_collection(TRAVEL, "visitedCities", &f.travels[0]));
    assert!(!f.contains_collection(TRAVEL, "visitedCities", &f.travels[1]));

    for city in &f.cities[..4] {
        assert!(!f.contains_entity(CITY, city));
    }

    let t1 = f.find(TRAVEL, id(&f.travels[0])).await.unwrap();
    let t2 = f.find(TRAVEL, id(&f.travels[1])).await.unwrap();

    let travel_region = f.get_entity_region(TRAVEL);
    let cities_region = f.get_collection_region(TRAVEL, "visitedCities");
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
    assert_eq!(2, f.logger.get_region_put_count(&travel_region));
    assert_eq!(2, f.logger.get_region_miss_count(&travel_region));

    //trigger lazy load
    assert_eq!(3, count(&t1, "visitedCities").await);
    assert_eq!(2, count(&t2, "visitedCities").await);

    assert_eq!(4, f.logger.get_put_count());
    assert_eq!(4, f.logger.get_miss_count());
    assert_eq!(2, f.logger.get_region_put_count(&cities_region));
    assert_eq!(2, f.logger.get_region_miss_count(&cities_region));

    for index in 0..3 {
        assert_instance_of(CITY, &get_at(&t1, "visitedCities", index).await);
    }
    for index in 0..2 {
        assert_instance_of(CITY, &get_at(&t2, "visitedCities", index).await);
    }

    assert!(f.contains_entity(TRAVEL, &f.travels[0]));
    assert!(f.contains_entity(TRAVEL, &f.travels[1]));

    assert!(f.contains_collection(TRAVEL, "visitedCities", &f.travels[0]));
    assert!(f.contains_collection(TRAVEL, "visitedCities", &f.travels[1]));

    for city in &f.cities[..4] {
        assert!(f.contains_entity(CITY, city));
    }

    f.em.clear().unwrap();
    f.logger.clear_stats();

    f.reset_query_log();

    let t3 = f.find(TRAVEL, id(&f.travels[0])).await.unwrap();
    let t4 = f.find(TRAVEL, id(&f.travels[1])).await.unwrap();

    //trigger lazy load from cache
    assert_eq!(3, count(&t3, "visitedCities").await);
    assert_eq!(2, count(&t4, "visitedCities").await);

    for index in 0..3 {
        assert_instance_of(CITY, &get_at(&t3, "visitedCities", index).await);
    }
    for index in 0..2 {
        assert_instance_of(CITY, &get_at(&t4, "visitedCities", index).await);
    }

    assert_eq!(4, f.logger.get_hit_count());
    assert_eq!(2, f.logger.get_region_hit_count(&travel_region));
    assert_eq!(2, f.logger.get_region_hit_count(&cities_region));

    for (old, new, len) in [(&t1, &t3, 3), (&t2, &t4, 2)] {
        for index in 0..len {
            let old = get_at(old, "visitedCities", index).await;
            let new = get_at(new, "visitedCities", index).await;
            assert!(!orm::same(&old, &new));
            assert_eq!(id(&old), id(&new));
            assert_eq!(name(&old), name(&new));
        }
    }

    assert_eq!(4, f.logger.get_hit_count());
    f.assert_query_count(0);
}

#[tokio::test]
async fn testStoreManyToManyAssociationWhitCascade() {
    let mut f = set_up().await;
    f.evict_regions();

    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_entity_region(TRAVELER).unwrap();
    f.cache.evict_entity_region(TRAVEL).unwrap();
    f.cache.evict_collection_region(STATE, "cities").unwrap();
    f.cache
        .evict_collection_region(TRAVELER, "travels")
        .unwrap();

    let traveler = traveler("Doctrine Bot");
    let travel = travel(&traveler);

    add_visited_city(&travel, &f.cities[0]);
    add_visited_city(&travel, &f.cities[1]);
    add_visited_city(&travel, &f.cities[3]);

    f.em.persist(&traveler).unwrap();
    f.em.persist(&travel).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(f.contains_entity(TRAVEL, &travel));
    assert!(f.contains_entity(TRAVELER, &traveler));
    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));
    assert!(f.contains_entity(CITY, &f.cities[3]));
    assert!(f.contains_collection(TRAVEL, "visitedCities", &travel));

    f.reset_query_log();
    let t1 = f.find(TRAVEL, id(&travel)).await.unwrap();

    assert_instance_of(TRAVEL, &t1);
    assert_eq!(3, count(&t1, "visitedCities").await);
    f.assert_query_count(0);
}

#[tokio::test]
async fn testReadOnlyCollection() {
    let mut f = set_up().await;
    f.evict_regions();

    load_travels(&mut f).await;

    f.em.clear().unwrap();

    assert!(f.contains_entity(TRAVEL, &f.travels[0]));
    assert!(f.contains_collection(TRAVEL, "visitedCities", &f.travels[0]));

    let travel = f.find(TRAVEL, id(&f.travels[0])).await.unwrap();

    assert_eq!(3, count(&travel, "visitedCities").await);

    coll::remove(&orm::get(&travel, "visitedCities"), 0)
        .await
        .unwrap();

    f.em.persist(&travel).unwrap();
    let e = f.em.flush().await.expect_err("expected a CacheException");
    assert!(
        php_reflection::is_a(&e.class, EXCEPTION_CACHE_EXCEPTION),
        "{}",
        e.class
    );
    assert!(
        orm::message(&e).contains(
            "Cannot update a readonly collection \"Doctrine\\Tests\\Models\\Cache\\Travel#visitedCities"
        ),
        "{}",
        orm::message(&e)
    );
}

#[tokio::test]
async fn testManyToManyWithEmptyRelation() {
    let mut f = set_up().await;
    load_travels(&mut f).await;
    f.em.clear().unwrap();

    f.evict_regions();

    f.reset_query_log();

    let entiti_id = id(&f.travels[2]); //empty travel
    let entity = f.find(TRAVEL, entiti_id.clone()).await.unwrap();

    assert_eq!(0, count(&entity, "visitedCities").await);
    f.assert_query_count(2);

    f.em.clear().unwrap();

    let entity = f.find(TRAVEL, entiti_id).await.unwrap();

    f.reset_query_log();
    assert_eq!(0, count(&entity, "visitedCities").await);
    f.assert_query_count(0);
}
