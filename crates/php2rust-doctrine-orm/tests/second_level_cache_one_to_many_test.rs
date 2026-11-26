//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheOneToManyTest.php

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

use php_value::Value;
use php2rust_doctrine_orm::collection_object as coll;

use slc_functional::{
    CITY, LOGIN, STATE, TOKEN, TRAVELER, add_login, assert_instance_of, count, get_at, id,
    is_initialized, login, name, set_up, token, travel, traveler,
};

#[tokio::test]
async fn testShouldPutCollectionInverseSideOnPersist() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.em.clear().unwrap();

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));
    assert!(f.contains_collection(STATE, "cities", &f.states[0]));
    assert!(f.contains_collection(STATE, "cities", &f.states[1]));
}

#[tokio::test]
async fn testPutAndLoadOneToManyRelation() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.em.clear().unwrap();
    f.logger.clear_stats();

    f.cache.evict_entity_region(STATE).unwrap();
    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_collection_region(STATE, "cities").unwrap();

    assert!(!f.contains_entity(STATE, &f.states[0]));
    assert!(!f.contains_entity(STATE, &f.states[1]));

    assert!(!f.contains_collection(STATE, "cities", &f.states[0]));
    assert!(!f.contains_collection(STATE, "cities", &f.states[1]));

    for state in &f.states[..2] {
        for index in 0..2 {
            assert!(!f.contains_entity(CITY, &get_at(state, "cities", index).await));
        }
    }

    let s1 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let s2 = f.find(STATE, id(&f.states[1])).await.unwrap();

    let state_region = f.get_entity_region(STATE);
    let cities_region = f.get_collection_region(STATE, "cities");
    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
    assert_eq!(2, f.logger.get_region_put_count(&state_region));
    assert_eq!(2, f.logger.get_region_miss_count(&state_region));

    //trigger lazy load
    assert_eq!(2, count(&s1, "cities").await);
    assert_eq!(2, count(&s2, "cities").await);

    assert_eq!(4, f.logger.get_put_count());
    assert_eq!(4, f.logger.get_miss_count());
    assert_eq!(2, f.logger.get_region_put_count(&cities_region));
    assert_eq!(2, f.logger.get_region_miss_count(&cities_region));

    for state in [&s1, &s2] {
        for index in 0..2 {
            assert_instance_of(CITY, &get_at(state, "cities", index).await);
        }
    }

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_entity(STATE, &f.states[1]));

    assert!(f.contains_collection(STATE, "cities", &f.states[0]));
    assert!(f.contains_collection(STATE, "cities", &f.states[1]));

    for state in &f.states[..2] {
        for index in 0..2 {
            assert!(f.contains_entity(CITY, &get_at(state, "cities", index).await));
        }
    }

    f.em.clear().unwrap();
    f.logger.clear_stats();

    f.reset_query_log();

    let s3 = f.find(STATE, id(&f.states[0])).await.unwrap();
    let s4 = f.find(STATE, id(&f.states[1])).await.unwrap();

    //trigger lazy load from cache
    assert_eq!(2, count(&s3, "cities").await);
    assert_eq!(2, count(&s4, "cities").await);

    assert_eq!(4, f.logger.get_hit_count());
    assert_eq!(2, f.logger.get_region_hit_count(&state_region));
    assert_eq!(2, f.logger.get_region_hit_count(&cities_region));

    for state in [&s3, &s4] {
        for index in 0..2 {
            assert_instance_of(CITY, &get_at(state, "cities", index).await);
        }
    }

    for (old, new) in [(&s1, &s3), (&s2, &s4)] {
        for index in 0..2 {
            let old = get_at(old, "cities", index).await;
            let new = get_at(new, "cities", index).await;
            assert!(!orm::same(&old, &new));
            assert_eq!(id(&old), id(&new));
            assert_eq!(name(&old), name(&new));
        }
    }

    assert_eq!(4, f.logger.get_hit_count());
    f.assert_query_count(0);
}

#[tokio::test]
async fn testLoadOneToManyCollectionFromDatabaseWhenEntityMissing() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.em.clear().unwrap();

    //trigger lazy load from database
    let state = f.find(STATE, id(&f.states[0])).await.unwrap();
    assert_eq!(2, count(&state, "cities").await);

    assert!(f.contains_entity(STATE, &f.states[0]));
    assert!(f.contains_collection(STATE, "cities", &f.states[0]));
    assert!(f.contains_entity(CITY, &get_at(&f.states[0], "cities", 0).await));
    assert!(f.contains_entity(CITY, &get_at(&f.states[0], "cities", 1).await));

    f.reset_query_log();
    let state_id = id(&f.states[0]);
    let state = f.find(STATE, state_id.clone()).await.unwrap();
    let city_id = id(&get_at(&f.states[0], "cities", 1).await);

    let contains = |class: &str, id: &Value| f.cache.contains_entity(class, id).unwrap();

    //trigger lazy load from cache
    assert_eq!(2, count(&state, "cities").await);
    f.assert_query_count(0);
    assert!(contains(CITY, &city_id));

    f.cache.evict_entity(CITY, &city_id).unwrap();

    assert!(!contains(CITY, &city_id));
    assert!(contains(STATE, &state_id));
    assert!(
        f.cache
            .contains_collection(STATE, "cities", &state_id)
            .unwrap()
    );

    f.em.clear().unwrap();

    let state = f.find(STATE, state_id).await.unwrap();

    //trigger lazy load from database
    assert_eq!(2, count(&state, "cities").await);
    f.assert_query_count(1);
}

#[tokio::test]
async fn testShoudNotPutOneToManyRelationOnPersist() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.evict_regions();

    let state = slc::state_in("State Foo", Some(&f.countries[0]));

    f.em.persist(&state).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(f.contains_entity(STATE, &state));
    assert!(!f.contains_collection(STATE, "cities", &state));
}

#[tokio::test]
async fn testOneToManyRemove() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.em.clear().unwrap();
    f.logger.clear_stats();

    f.cache.evict_entity_region(STATE).unwrap();
    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_collection_region(STATE, "cities").unwrap();

    assert!(!f.contains_entity(STATE, &f.states[0]));
    assert!(!f.contains_collection(STATE, "cities", &f.states[0]));
    assert!(!f.contains_entity(CITY, &get_at(&f.states[0], "cities", 0).await));
    assert!(!f.contains_entity(CITY, &get_at(&f.states[0], "cities", 1).await));

    let entity = f.find(STATE, id(&f.states[0])).await.unwrap();

    let state_region = f.get_entity_region(STATE);
    let cities_region = f.get_collection_region(STATE, "cities");
    assert_eq!(1, f.logger.get_put_count());
    assert_eq!(1, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&state_region));
    assert_eq!(1, f.logger.get_region_miss_count(&state_region));

    //trigger lazy load
    assert_eq!(2, count(&entity, "cities").await);

    assert_eq!(2, f.logger.get_put_count());
    assert_eq!(2, f.logger.get_miss_count());
    assert_eq!(1, f.logger.get_region_put_count(&cities_region));
    assert_eq!(1, f.logger.get_region_miss_count(&cities_region));

    assert_instance_of(CITY, &get_at(&entity, "cities", 0).await);
    assert_instance_of(CITY, &get_at(&entity, "cities", 1).await);

    f.em.clear().unwrap();
    f.logger.clear_stats();

    f.reset_query_log();
    let state = f.find(STATE, id(&f.states[0])).await.unwrap();

    //trigger lazy load from cache
    assert_eq!(2, count(&state, "cities").await);

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_region_hit_count(&state_region));
    assert_eq!(1, f.logger.get_region_hit_count(&cities_region));

    let city0 = get_at(&state, "cities", 0).await;
    let city1 = get_at(&state, "cities", 1).await;

    assert_instance_of(CITY, &city0);
    assert_instance_of(CITY, &city1);

    assert_eq!(name(&get_at(&entity, "cities", 0).await), name(&city0));
    assert_eq!(name(&get_at(&entity, "cities", 1).await), name(&city1));

    assert_eq!(2, f.logger.get_hit_count());
    f.assert_query_count(0);

    orm::remove_element(&state, "cities", &city0).await;

    f.em.remove(&city0).await.unwrap();
    f.em.persist(&state).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.logger.clear_stats();

    f.reset_query_log();
    let state = f.find(STATE, id(&f.states[0])).await.unwrap();

    //trigger lazy load from cache
    assert_eq!(1, count(&state, "cities").await);

    let city1 = get_at(&state, "cities", 0).await;
    assert_instance_of(CITY, &city1);
    assert_eq!(name(&get_at(&entity, "cities", 1).await), name(&city1));

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_region_hit_count(&state_region));
    assert_eq!(1, f.logger.get_region_hit_count(&cities_region));

    f.assert_query_count(0);

    coll::remove(&orm::get(&state, "cities"), 0).await.unwrap();

    f.em.remove(&city1).await.unwrap();
    f.em.persist(&state).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.logger.clear_stats();

    f.reset_query_log();
    let state = f.find(STATE, id(&f.states[0])).await.unwrap();

    assert_eq!(0, count(&state, "cities").await);

    assert_eq!(2, f.logger.get_hit_count());
    assert_eq!(1, f.logger.get_region_hit_count(&state_region));
    assert_eq!(1, f.logger.get_region_hit_count(&cities_region));
}

#[tokio::test]
async fn testOneToManyWithEmptyRelation() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.logger.clear_stats();
    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_entity_region(STATE).unwrap();
    f.cache.evict_collection_region(STATE, "cities").unwrap();
    f.em.clear().unwrap();

    let entiti_id = id(&f.states[2]); // bavaria (cities count = 0)
    f.reset_query_log();
    let entity = f.find(STATE, entiti_id.clone()).await.unwrap();

    assert_eq!(0, count(&entity, "cities").await);
    f.assert_query_count(2);

    f.em.clear().unwrap();

    f.reset_query_log();
    let entity = f.find(STATE, entiti_id).await.unwrap();

    assert_eq!(0, count(&entity, "cities").await);
    f.assert_query_count(0);
}

#[tokio::test]
async fn testOneToManyCount() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.logger.clear_stats();
    f.cache.evict_entity_region(CITY).unwrap();
    f.cache.evict_entity_region(STATE).unwrap();
    f.cache.evict_collection_region(STATE, "cities").unwrap();
    f.em.clear().unwrap();

    let entity_id = id(&f.states[0]);
    f.reset_query_log();
    let entity = f.find(STATE, entity_id.clone()).await.unwrap();

    assert_eq!(2, count(&entity, "cities").await);
    f.assert_query_count(2);

    f.em.clear().unwrap();

    f.reset_query_log();
    let entity = f.find(STATE, entity_id).await.unwrap();

    assert_eq!(2, count(&entity, "cities").await);
    f.assert_query_count(0);
}

#[tokio::test]
async fn testCacheInitializeCollectionWithNewObjects() {
    let f = set_up().await;
    f.em.clear().unwrap();

    f.evict_regions();

    let traveler = traveler("Doctrine Bot");

    for _ in 0..3 {
        coll::add(
            &orm::get(&traveler, "travels"),
            Value::Object(travel(&traveler)),
        )
        .unwrap();
    }

    f.em.persist(&traveler).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert_eq!(3, count(&traveler, "travels").await);

    let traveler_id = id(&traveler);
    f.reset_query_log();
    let entity = f.find(TRAVELER, traveler_id.clone()).await.unwrap();

    f.assert_query_count(0);
    assert!(!is_initialized(&entity, "travels"));

    let new_item = travel(&entity);
    coll::add(&orm::get(&entity, "travels"), Value::Object(new_item)).unwrap();

    assert!(!is_initialized(&entity, "travels"));
    assert_eq!(4, count(&entity, "travels").await);
    assert!(is_initialized(&entity, "travels"));
    f.assert_query_count(0);

    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    let query = format!(
        "SELECT t, tt FROM Doctrine\\Tests\\Models\\Cache\\Traveler t JOIN t.travels tt WHERE t.id = {}",
        traveler_id.to_php_string().unwrap().to_string_lossy()
    );
    let result = orm::obj(
        &f.em
            .create_query(&query)
            .get_single_result(None)
            .await
            .unwrap(),
    );

    assert_eq!(4, count(&result, "travels").await);
}

#[tokio::test]
async fn testPutAndLoadNonCacheableOneToMany() {
    let f = set_up().await;
    assert!(f.cache.get_entity_cache_region(LOGIN).unwrap().is_none());
    assert!(f.cache.get_entity_cache_region(TOKEN).unwrap().is_some());

    let l1 = login("session1");
    let l2 = login("session2");
    let token = token("token-hash", None);
    add_login(&token, &l1);
    add_login(&token, &l2);

    f.em.persist(&token).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(
        f.cache
            .contains_entity(TOKEN, &orm::get(&token, "token"))
            .unwrap()
    );

    f.reset_query_log();

    let entity = f.find(TOKEN, orm::get(&token, "token")).await.unwrap();

    assert_instance_of(TOKEN, &entity);
    assert_eq!(Value::string("token-hash"), orm::get(&entity, "token"));
    f.assert_query_count(0);

    assert_eq!(2, count(&entity, "logins").await);
    f.assert_query_count(1);
}
