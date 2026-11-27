//! Ticket/DDC1884Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the taxi models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1884Test.php

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
#[path = "common/taxi.rs"]
mod taxi;

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{list, s};
use taxi::*;

async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&TAXI_MODELS).await;

    let bimmer = car("BMW", "7-Series");
    let crysler = car("Crysler", "300");
    let merc = car("Mercedes", "C-Class");
    let volvo = car("Volvo", "XC90");
    for car in [&bimmer, &crysler, &merc, &volvo] {
        em.persist(car).unwrap();
    }

    let john = driver("John Doe");
    let foo_bar = driver("Foo Bar");
    em.persist(&foo_bar).unwrap();
    em.persist(&john).unwrap();
    em.flush().await.unwrap();

    for (driver, car) in [
        (&john, &bimmer),
        (&john, &merc),
        (&john, &volvo),
        (&foo_bar, &merc),
    ] {
        em.persist(&ride(RIDE, driver, car)).unwrap();
    }

    for (driver, car, fare) in [
        (&john, &bimmer, 10.50),
        (&john, &merc, 16.00),
        (&john, &volvo, 20.70),
        (&foo_bar, &merc, 32.15),
    ] {
        let paid = ride(PAID_RIDE, driver, car);
        orm::set(&paid, "fare", Value::Float(fare));
        em.persist(&paid).unwrap();
    }

    em.flush().await.unwrap();
    em
}

fn row(value: &Value) -> PhpArray {
    match value {
        Value::Array(array) => array.clone(),
        other => panic!("{other:?} is not an array"),
    }
}

async fn select_driver_with(collection: &str) -> Vec<Value> {
    let em = set_up().await;
    let mut qb = em.create_query_builder();
    qb.select(vec![s("d, dr, c")])
        .unwrap()
        .from(DRIVER, "d", None)
        .unwrap()
        .left_join(&format!("d.{collection}"), "dr", None, None, None)
        .unwrap()
        .left_join("dr.car", "c", None, None, None)
        .unwrap()
        .r#where(vec![s("d.name = ?1")])
        .unwrap()
        .set_parameter(1, s("John Doe"), None);
    list(&qb.get_query().unwrap().get_array_result().await.unwrap())
}

/// 1) `Ride` contains only columns that are part of its composite primary key
/// 2) We use fetch joins here
#[tokio::test]
async fn testSelectFromInverseSideWithCompositePkAndSolelyIdentifierColumnsUsingFetchJoins() {
    let result = select_driver_with("freeDriverRides").await;

    assert_eq!(1, result.len());
    let rides = row(&result[0]).get(&"freeDriverRides".into()).cloned();
    assert!(rides.is_some(), "has key freeDriverRides");
    assert_eq!(3, list(&rides.unwrap()).len());
}

/// 1) `PaidRide` contains an extra column that is not part of the composite primary key
/// 2) Again we will use fetch joins
#[tokio::test]
async fn testSelectFromInverseSideWithCompositePkUsingFetchJoins() {
    let result = select_driver_with("driverRides").await;

    assert_eq!(1, result.len());
    let rides = row(&result[0]).get(&"driverRides".into()).cloned();
    assert!(rides.is_some(), "has key driverRides");
    assert_eq!(3, list(&rides.unwrap()).len());
}

/// The other way around will fail too
#[tokio::test]
async fn testSelectFromOwningSideUsingFetchJoins() {
    let em = set_up().await;
    let mut qb = em.create_query_builder();
    qb.select(vec![s("r, d, c")])
        .unwrap()
        .from(PAID_RIDE, "r", None)
        .unwrap()
        .left_join("r.driver", "d", None, None, None)
        .unwrap()
        .left_join("r.car", "c", None, None, None)
        .unwrap()
        .r#where(vec![s("d.name = ?1")])
        .unwrap()
        .set_parameter(1, s("John Doe"), None);
    let result = list(&qb.get_query().unwrap().get_array_result().await.unwrap());

    assert_eq!(3, result.len());
    assert!(row(&result[0]).get(&"driver".into()).is_some());
    assert!(row(&result[0]).get(&"car".into()).is_some());
}
