//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SecondLevelCacheCompositePrimaryKeyTest.php

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

use php_value::{Object, Value};

use slc_functional::{
    CITY, FLIGHT, Fixture, assert_instance_of, flight, get_one, id, now, set_up, tomorrow,
};

/// The fixtures, then a new flight from the first city to the second:
/// the flight, its id and the cities' ids.
async fn set_up_flight(f: &mut Fixture) -> (Object, Value, Value, Value) {
    f.load_fixtures_countries().await;
    f.load_fixtures_states().await;
    f.load_fixtures_cities().await;

    f.em.clear().unwrap();
    f.evict_regions();

    let leaving_from_id = id(&f.cities[0]);
    let going_to_id = id(&f.cities[1]);
    let leaving_from = f.find(CITY, leaving_from_id.clone()).await.unwrap();
    let going_to = f.find(CITY, going_to_id.clone()).await.unwrap();
    let flight = flight(&leaving_from, &going_to);
    let id = Value::Array(slc::array(&[
        ("leavingFrom", leaving_from_id.clone()),
        ("goingTo", going_to_id.clone()),
    ]));
    (flight, id, leaving_from_id, going_to_id)
}

fn contains(f: &Fixture, class: &str, id: &Value) -> bool {
    f.cache.contains_entity(class, id).unwrap()
}

/// `$this->_em->find(Flight::class, $id)` and the assertions on its
/// cities.
async fn assert_flight(
    f: &Fixture,
    id: &Value,
    leaving_from_id: &Value,
    going_to_id: &Value,
) -> Object {
    let flight = f.find(FLIGHT, id.clone()).await.unwrap();
    let leaving_from = get_one(&flight, "leavingFrom");
    let going_to = get_one(&flight, "goingTo");

    assert_instance_of(FLIGHT, &flight);
    assert_instance_of(CITY, &going_to);
    assert_instance_of(CITY, &leaving_from);

    assert_eq!(slc_functional::id(&going_to), *going_to_id);
    assert_eq!(slc_functional::id(&leaving_from), *leaving_from_id);
    flight
}

#[tokio::test]
async fn testPutAndLoadCompositPrimaryKeyEntities() {
    let mut f = set_up().await;
    let (flight, id, leaving_from_id, going_to_id) = set_up_flight(&mut f).await;

    orm::set(&flight, "departure", tomorrow());

    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.em.persist(&flight).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(contains(&f, FLIGHT, &id));
    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.reset_query_log();
    assert_flight(&f, &id, &leaving_from_id, &going_to_id).await;
    f.assert_query_count(0);
}

#[tokio::test]
async fn testRemoveCompositPrimaryKeyEntities() {
    let mut f = set_up().await;
    let (flight, id, _, _) = set_up_flight(&mut f).await;

    orm::set(&flight, "departure", tomorrow());

    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.em.persist(&flight).unwrap();
    f.em.flush().await.unwrap();

    assert!(contains(&f, FLIGHT, &id));
    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.em.remove(&flight).await.unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(!contains(&f, FLIGHT, &id));
    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    assert!(f.find(FLIGHT, id).await.is_none());
}

#[tokio::test]
async fn testUpdateCompositPrimaryKeyEntities() {
    let mut f = set_up().await;
    let now = now();
    let tomorrow = tomorrow();
    let (flight, id, leaving_from_id, going_to_id) = set_up_flight(&mut f).await;

    orm::set(&flight, "departure", now.clone());

    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.em.persist(&flight).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(contains(&f, FLIGHT, &id));
    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.reset_query_log();
    let flight = assert_flight(&f, &id, &leaving_from_id, &going_to_id).await;
    orm::assert_loose_eq(&orm::get(&flight, "departure"), &now);
    f.assert_query_count(0);

    orm::set(&flight, "departure", tomorrow.clone());

    f.em.persist(&flight).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    assert!(contains(&f, FLIGHT, &id));
    assert!(f.contains_entity(CITY, &f.cities[0]));
    assert!(f.contains_entity(CITY, &f.cities[1]));

    f.reset_query_log();
    let flight = assert_flight(&f, &id, &leaving_from_id, &going_to_id).await;
    orm::assert_loose_eq(&orm::get(&flight, "departure"), &tomorrow);
    f.assert_query_count(0);
}
