//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheExtraLazyCollectionTest.php
//!
//! Each test has its own entity manager and metadata, so `tearDown()`'s
//! reset of the fetch modes has nothing to undo.

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use php_value::Value;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EXTRA_LAZY;

use slc_functional::{CITY, Fixture, STATE, TRAVEL, id, is_initialized, set_up};

/// `setUp()`: the travels' cities and the cities' travels are extra lazy.
async fn set_up_extra_lazy() -> Fixture {
    let f = set_up().await;
    let source_entity = f.em.get_class_metadata(TRAVEL).unwrap();
    let target_entity = f.em.get_class_metadata(CITY).unwrap();

    source_entity
        .write()
        .association_mappings
        .get_mut("visitedCities")
        .unwrap()
        .fetch = Some(FETCH_EXTRA_LAZY);
    target_entity
        .write()
        .association_mappings
        .get_mut("travels")
        .unwrap()
        .fetch = Some(FETCH_EXTRA_LAZY);
    f
}

#[tokio::test]
async fn testCacheCountAfterAddThenFlush() {
    let mut f = set_up_extra_lazy().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.load_fixtures_traveler().await;
    f.load_fixtures_travels().await;

    f.em.clear().unwrap();

    let owner_id = id(&f.travels[0]);
    let owner = f.find(TRAVEL, owner_id.clone()).await.unwrap();
    let r#ref = f.find(STATE, id(&f.states[1])).await.unwrap();

    assert!(f.cache.contains_entity(TRAVEL, &owner_id).unwrap());
    assert!(
        f.cache
            .contains_collection(TRAVEL, "visitedCities", &owner_id)
            .unwrap()
    );

    let new_item = slc::city("New City", Some(&r#ref));
    coll::add(
        &orm::get(&owner, "visitedCities"),
        Value::Object(new_item.clone()),
    )
    .unwrap();

    f.em.persist(&new_item).unwrap();
    f.em.persist(&owner).unwrap();

    f.reset_query_log();

    assert!(!is_initialized(&owner, "visitedCities"));
    assert_eq!(4, slc_functional::count(&owner, "visitedCities").await);
    assert!(!is_initialized(&owner, "visitedCities"));
    f.assert_query_count(0);

    f.em.flush().await.unwrap();

    assert!(!is_initialized(&owner, "visitedCities"));
    assert!(
        !f.cache
            .contains_collection(TRAVEL, "visitedCities", &owner_id)
            .unwrap()
    );

    f.em.clear().unwrap();

    f.reset_query_log();
    let owner = f.find(TRAVEL, owner_id).await.unwrap();

    assert_eq!(4, slc_functional::count(&owner, "visitedCities").await);
    assert!(!is_initialized(&owner, "visitedCities"));
    f.assert_query_count(1);
}
