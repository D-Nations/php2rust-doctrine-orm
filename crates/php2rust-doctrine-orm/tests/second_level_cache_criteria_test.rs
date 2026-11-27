//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheCriteriaTest.php
//!
//! `Criteria::create(true)` is `Criteria::create()`; `$result[0]` of the
//! repository's lazy collection is its `get(0)`.

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

use std::sync::Arc;

use php_value::{Key, Object, Value};
use php2rust_doctrine_collections::Criteria;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::lazy_criteria_collection::LazyCriteriaCollection;

use slc_functional::{COUNTRY, STATE, assert_instance_of, get_at, id, name, set_up};

fn criteria_eq(field: &str, value: Value) -> Criteria {
    let mut criteria = Criteria::create();
    criteria.r#where(Criteria::expr().eq(field, value));
    criteria
}

/// `$result[$index]`.
async fn at(result: &Arc<LazyCriteriaCollection>, index: i64) -> Object {
    orm::obj(&result.get(&Key::Int(index)).await.unwrap())
}

#[tokio::test]
async fn testMatchingPut() {
    let mut f = set_up().await;
    f.evict_regions();

    f.load_fixtures_countries().await;
    f.evict_regions();
    f.em.clear().unwrap();

    assert!(!f.contains_entity(COUNTRY, &f.countries[0]));

    let repository = f.em.get_repository(COUNTRY).unwrap();
    f.reset_query_log();
    let country_name = name(&f.countries[0]);
    let result1 = repository
        .matching(&criteria_eq("name", country_name.clone()))
        .unwrap();

    // Because matching returns lazy collection, we force initialization
    result1.to_array().await.unwrap();

    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&at(&result1, 0).await));
    assert_eq!(name(&f.countries[0]), name(&at(&result1, 0).await));

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));

    f.em.clear().unwrap();

    let result2 = repository
        .matching(&criteria_eq("name", country_name))
        .unwrap();

    f.assert_query_count(1);
    assert_eq!(1, result2.count().await.unwrap());

    assert_instance_of(COUNTRY, &at(&result2, 0).await);

    assert_eq!(id(&at(&result1, 0).await), id(&at(&result2, 0).await));
    assert_eq!(name(&at(&result1, 0).await), name(&at(&result2, 0).await));
}

#[tokio::test]
async fn testRepositoryMatching() {
    let mut f = set_up().await;
    f.evict_regions();

    f.load_fixtures_countries().await;
    f.em.clear().unwrap();

    assert!(f.contains_entity(COUNTRY, &f.countries[0]));

    let repository = f.em.get_repository(COUNTRY).unwrap();
    f.reset_query_log();
    let result1 = repository
        .matching(&criteria_eq("name", name(&f.countries[0])))
        .unwrap();

    // Because matching returns lazy collection, we force initialization
    result1.to_array().await.unwrap();

    assert_eq!(1, result1.count().await.unwrap());
    f.assert_query_count(1);
    assert_eq!(id(&f.countries[0]), id(&at(&result1, 0).await));
    assert_eq!(name(&f.countries[0]), name(&at(&result1, 0).await));

    f.em.clear().unwrap();

    let result2 = repository
        .matching(&criteria_eq("name", name(&f.countries[0])))
        .unwrap();

    // Because matching returns lazy collection, we force initialization
    result2.to_array().await.unwrap();

    f.assert_query_count(1);
    assert_eq!(1, result2.count().await.unwrap());

    assert_instance_of(COUNTRY, &at(&result2, 0).await);

    assert_eq!(id(&f.countries[0]), id(&at(&result2, 0).await));
    assert_eq!(name(&f.countries[0]), name(&at(&result2, 0).await));

    let result3 = repository
        .matching(&criteria_eq("name", name(&f.countries[1])))
        .unwrap();

    // Because matching returns lazy collection, we force initialization
    result3.to_array().await.unwrap();

    f.assert_query_count(2);
    assert_eq!(1, result3.count().await.unwrap());

    assert_instance_of(COUNTRY, &at(&result3, 0).await);

    assert_eq!(id(&f.countries[1]), id(&at(&result3, 0).await));
    assert_eq!(name(&f.countries[1]), name(&at(&result3, 0).await));

    let result4 = repository
        .matching(&criteria_eq("name", name(&f.countries[1])))
        .unwrap();

    f.assert_query_count(2);
    // The query count above is asserted before `assertCount()`, as
    // upstream does (the count of a lazy collection is a COUNT query).
    assert_eq!(1, result4.count().await.unwrap());

    assert_instance_of(COUNTRY, &at(&result4, 0).await);

    assert_eq!(id(&f.countries[1]), id(&at(&result4, 0).await));
    assert_eq!(name(&f.countries[1]), name(&at(&result4, 0).await));
}

#[tokio::test]
async fn testCollectionMatching() {
    let mut f = set_up().await;
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;
    f.em.clear().unwrap();
    f.logger.clear_stats();

    let entity = f.find(STATE, id(&f.states[0])).await.unwrap();
    let item_name = name(&get_at(&f.states[0], "cities", 0).await);
    f.reset_query_log();
    let collection = orm::get(&entity, "cities");
    let matching = coll::matching(&collection, &criteria_eq("name", item_name.clone()))
        .await
        .unwrap();

    f.assert_query_count(1);
    assert_eq!(1, coll::count(&matching).await.unwrap());

    f.em.clear().unwrap();

    let entity = f.find(STATE, id(&f.states[0])).await.unwrap();
    f.reset_query_log();
    let collection = orm::get(&entity, "cities");
    let matching = coll::matching(&collection, &criteria_eq("name", item_name))
        .await
        .unwrap();

    f.assert_query_count(0);
    assert_eq!(1, coll::count(&matching).await.unwrap());
}
