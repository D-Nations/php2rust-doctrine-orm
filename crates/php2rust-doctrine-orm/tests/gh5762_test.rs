//! Ticket/GH5762Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5762Test.php

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
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use ticket::*;

/// `new GH5762Car($brand, $model)`.
fn car(brand: &str, model: &str) -> Object {
    let car = new("GH5762Car");
    set(&car, "brand", s(brand));
    set(&car, "model", s(model));
    set(
        &car,
        "carRides",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    car
}

/// `new GH5762Driver($id, $name)`.
fn driver(id: i64, name: &str) -> Object {
    let driver = new("GH5762Driver");
    set(&driver, "id", Value::Int(id));
    set(&driver, "name", s(name));
    set(
        &driver,
        "driverRides",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    driver
}

/// `new GH5762DriverRide($driver, $car)`.
fn driver_ride(driver: &Object, car: &Object) -> Object {
    let ride = new("GH5762DriverRide");
    set_obj(&ride, "driver", driver);
    set_obj(&ride, "car", car);
    coll::add(&get(driver, "driverRides"), Value::Object(ride.clone())).unwrap();
    coll::add(&get(car, "carRides"), Value::Object(ride.clone())).unwrap();
    ride
}

/// `createData()`.
async fn create_data(em: &EntityManager) {
    let car1 = car("BMW", "7 Series");
    let car2 = car("Crysler", "300");
    let car3 = car("Dodge", "Dart");
    let car4 = car("Mercedes", "C-Class");
    let car5 = car("Volvo", "XC90");

    let driver = driver(1, "John Doe");

    let ride1 = driver_ride(&driver, &car1);
    let ride2 = driver_ride(&driver, &car2);
    let ride3 = driver_ride(&driver, &car3);
    let ride4 = driver_ride(&driver, &car4);
    let ride5 = driver_ride(&driver, &car5);

    for entity in [&car1, &car2, &car3, &car4, &car5, &driver] {
        em.persist(entity).unwrap();
    }
    for entity in [&ride1, &ride2, &ride3, &ride4, &ride5] {
        em.persist(entity).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["GH5762Driver", "GH5762DriverRide", "GH5762Car"]).await;

    // fetchData()
    create_data(&em).await;

    let mut qb = em.create_query_builder();
    qb.select(vec![s("d, dr, c")])
        .unwrap()
        .from(&class("GH5762Driver"), "d", None)
        .unwrap()
        .left_join("d.driverRides", "dr", None, None, None)
        .unwrap()
        .left_join("dr.car", "c", None, None, None)
        .unwrap()
        .r#where(vec![s("d.id = 1")])
        .unwrap();

    let result = qb
        .get_query()
        .unwrap()
        .get_single_result(None)
        .await
        .unwrap();

    assert_instance_of("GH5762Driver", &result);
    let result = obj(&result);
    let driver_rides = get(&result, "driverRides");
    assert!(PersistentCollection::from_value(&driver_rides).is_some());
    let first = coll::get(&driver_rides, 0).await.unwrap();
    assert_instance_of("GH5762DriverRide", &first);
    assert_instance_of("GH5762Car", &get(&obj(&first), "car"));

    let mut cars = Vec::new();
    for ride in coll::get_values(&driver_rides).await.unwrap() {
        cars.push(get(&obj(&get(&obj(&ride), "car")), "brand"));
    }

    // count(array_unique($cars)): the brands are strings
    let distinct = cars
        .iter()
        .enumerate()
        .filter(|(i, car)| !cars[..*i].contains(car))
        .count();
    assert_eq!(cars.len(), distinct);

    for brand in ["BMW", "Crysler", "Dodge", "Mercedes", "Volvo"] {
        assert!(cars.contains(&s(brand)), "{cars:?}");
    }
}
