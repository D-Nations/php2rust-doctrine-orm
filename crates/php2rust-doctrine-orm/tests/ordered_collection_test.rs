//! Port of `OrderedCollectionTest`, on `SQLite` with the `routing` model
//! set. `getName()` and `getPassengerName()` read the properties they
//! return; the collections are read through `elements()`, which
//! initializes them as PHP's array access does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OrderedCollectionTest.php

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

use std::collections::HashMap;
use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{append, assert_loose_eq, elements, get, get_object, new_object, obj, s, set};

const ROUTING_LEG: &str = "Doctrine\\Tests\\Models\\Routing\\RoutingLeg";
const ROUTING_LOCATION: &str = "Doctrine\\Tests\\Models\\Routing\\RoutingLocation";
const ROUTING_ROUTE: &str = "Doctrine\\Tests\\Models\\Routing\\RoutingRoute";
const ROUTING_ROUTE_BOOKING: &str = "Doctrine\\Tests\\Models\\Routing\\RoutingRouteBooking";

struct Test {
    em: Arc<EntityManager>,
    locations: HashMap<&'static str, Object>,
}

/// `setUp()`.
async fn set_up() -> Test {
    let em = orm::set_up(&[
        ROUTING_LEG,
        ROUTING_LOCATION,
        ROUTING_ROUTE,
        ROUTING_ROUTE_BOOKING,
    ])
    .await;

    let mut locations = HashMap::new();
    for location_name in ["Berlin", "Bonn", "Brasilia", "Atlanta"] {
        let location = new_object(ROUTING_LOCATION);
        set(&location, "name", s(location_name));
        em.persist(&location).unwrap();
        locations.insert(location_name, location);
    }

    em.flush().await.unwrap();
    Test { em, locations }
}

/// `new RoutingRoute()`.
fn routing_route() -> Object {
    let route = new_object(ROUTING_ROUTE);
    set(
        &route,
        "legs",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    route
}

/// `new DateTime("now +$hours hours")`.
fn now_plus_hours(hours: i64) -> Value {
    let now = DateTimeImmutable::now(TimeZone::Utc).timestamp();
    DateTime::object(DateTimeImmutable::from_timestamp(
        now + hours * 3600,
        0,
        TimeZone::Utc,
    ))
}

impl Test {
    /// `createPersistedRouteWithLegs()`.
    async fn create_persisted_route_with_legs(&self) -> Value {
        let route = routing_route();

        let leg1 = new_object(ROUTING_LEG);
        set(
            &leg1,
            "fromLocation",
            Value::Object(self.locations["Berlin"].clone()),
        );
        set(
            &leg1,
            "toLocation",
            Value::Object(self.locations["Bonn"].clone()),
        );
        set(&leg1, "departureDate", now_plus_hours(0));
        set(&leg1, "arrivalDate", now_plus_hours(5));

        let leg2 = new_object(ROUTING_LEG);
        set(
            &leg2,
            "fromLocation",
            Value::Object(self.locations["Bonn"].clone()),
        );
        set(
            &leg2,
            "toLocation",
            Value::Object(self.locations["Brasilia"].clone()),
        );
        set(&leg2, "departureDate", now_plus_hours(6));
        set(&leg2, "arrivalDate", now_plus_hours(24));

        append(&route, "legs", Value::Object(leg2));
        append(&route, "legs", Value::Object(leg1));

        self.em.persist(&route).unwrap();
        self.em.flush().await.unwrap();
        let route_id = get(&route, "id");
        self.em.clear().unwrap();

        route_id
    }
}

/// `$leg->fromLocation->getName()`.
async fn from_location_name(em: &EntityManager, leg: &Object) -> Value {
    let location = get_object(leg, "fromLocation").unwrap();
    em.initialize_object(&location).await.unwrap();
    get(&location, "name")
}

#[tokio::test]
async fn testLazyManyToManyCollectionIsRetrievedWithOrderByClause() {
    let test = set_up().await;
    let route_id = test.create_persisted_route_with_legs().await;

    let route = test
        .em
        .find(ROUTING_ROUTE, route_id)
        .await
        .unwrap()
        .unwrap();

    let legs = elements(&route, "legs").await;
    assert_eq!(2, legs.len());
    assert_loose_eq(&s("Berlin"), &from_location_name(&test.em, &legs[0]).await);
    assert_loose_eq(&s("Bonn"), &from_location_name(&test.em, &legs[1]).await);
}

#[tokio::test]
async fn testLazyOneToManyCollectionIsRetrievedWithOrderByClause() {
    let test = set_up().await;
    let em = &test.em;
    let route = routing_route();

    em.persist(&route).unwrap();
    em.flush().await.unwrap();
    let route_id = get(&route, "id");

    let booking1 = new_object(ROUTING_ROUTE_BOOKING);
    set(&booking1, "passengerName", s("Guilherme"));
    let booking2 = new_object(ROUTING_ROUTE_BOOKING);
    set(&booking2, "passengerName", s("Benjamin"));

    append(&route, "bookings", Value::Object(booking1.clone()));
    set(&booking1, "route", Value::Object(route.clone()));
    append(&route, "bookings", Value::Object(booking2.clone()));
    set(&booking2, "route", Value::Object(route.clone()));

    em.persist(&booking1).unwrap();
    em.persist(&booking2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let route = em.find(ROUTING_ROUTE, route_id).await.unwrap().unwrap();

    let route_bookings = elements(&route, "bookings").await;
    assert_eq!(2, route_bookings.len());
    assert_loose_eq(&s("Benjamin"), &get(&route_bookings[0], "passengerName"));
    assert_loose_eq(&s("Guilherme"), &get(&route_bookings[1], "passengerName"));
}

#[tokio::test]
async fn testOrderedResultFromDqlQuery() {
    let test = set_up().await;
    let route_id = test.create_persisted_route_with_legs().await;

    let query = test.em.create_query(
        "SELECT r, l FROM Doctrine\\Tests\\Models\\Routing\\RoutingRoute r JOIN r.legs l WHERE r.id = ?1",
    );
    query.set_parameter(1, route_id, None);
    let route = obj(&query.get_single_result(None).await.unwrap());

    let legs = elements(&route, "legs").await;
    assert_eq!(2, legs.len());
    assert_loose_eq(&s("Berlin"), &from_location_name(&test.em, &legs[0]).await);
    assert_loose_eq(&s("Bonn"), &from_location_name(&test.em, &legs[1]).await);
}
