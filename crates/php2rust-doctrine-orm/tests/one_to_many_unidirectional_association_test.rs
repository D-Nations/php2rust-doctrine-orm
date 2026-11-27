//! Upstream: doctrine/orm tests/Tests/ORM/Functional/OneToManyUnidirectionalAssociationTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::*;

const ROUTING: &str = "Doctrine\\Tests\\Models\\Routing\\";

fn class(name: &str) -> String {
    format!("{ROUTING}{name}")
}

/// `setUp()`: the schema and the locations Berlin, Bonn, Brasilia and
/// Atlanta.
async fn set_up_locations() -> (
    std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
    Vec<php_value::Object>,
) {
    let models = [
        class("RoutingLeg"),
        class("RoutingLocation"),
        class("RoutingRoute"),
        class("RoutingRouteBooking"),
    ];
    let models: Vec<&str> = models.iter().map(String::as_str).collect();
    let em = set_up(&models).await;

    let mut locations = Vec::new();
    for location_name in ["Berlin", "Bonn", "Brasilia", "Atlanta"] {
        let location = new_object(&class("RoutingLocation"));
        set(&location, "name", s(location_name));
        em.persist(&location).unwrap();
        locations.push(location);
    }
    em.flush().await.unwrap();
    (em, locations)
}

/// A `RoutingLeg` from Berlin to Bonn, departing now, arriving in five
/// hours.
fn berlin_bonn_leg(locations: &[php_value::Object]) -> php_value::Object {
    let leg = new_object(&class("RoutingLeg"));
    set_obj(&leg, "fromLocation", &locations[0]);
    set_obj(&leg, "toLocation", &locations[1]);
    let now = DateTimeImmutable::now(TimeZone::Utc);
    set(&leg, "departureDate", DateTime::object(now));
    set(
        &leg,
        "arrivalDate",
        DateTime::object(DateTimeImmutable::from_timestamp(
            now.timestamp() + 5 * 3600,
            0,
            TimeZone::Utc,
        )),
    );
    leg
}

#[tokio::test]
async fn testPersistOwningInverseCascade() {
    let (em, locations) = set_up_locations().await;
    let leg = berlin_bonn_leg(&locations);

    let route = new_object(&class("RoutingRoute"));
    set(
        &route,
        "legs",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    append(&route, "legs", Value::Object(leg));

    em.persist(&route).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let routes = obj(&em
        .create_query(
            "SELECT r, l, f, t FROM Doctrine\\Tests\\Models\\Routing\\RoutingRoute r \
             JOIN r.legs l JOIN l.fromLocation f JOIN l.toLocation t",
        )
        .get_single_result(None)
        .await
        .unwrap());

    let legs = elements(&routes, "legs").await;
    assert_eq!(legs.len(), 1);
    assert_loose_eq(
        &s("Berlin"),
        &get(&get_object(&legs[0], "fromLocation").unwrap(), "name"),
    );
    assert_loose_eq(
        &s("Bonn"),
        &get(&get_object(&legs[0], "toLocation").unwrap(), "name"),
    );
}

#[tokio::test]
async fn testLegsAreUniqueToRoutes() {
    let (em, locations) = set_up_locations().await;

    let leg = berlin_bonn_leg(&locations);

    let route_a = new_object(&class("RoutingRoute"));
    set(
        &route_a,
        "legs",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    append(&route_a, "legs", Value::Object(leg.clone()));

    let route_b = new_object(&class("RoutingRoute"));
    set(
        &route_b,
        "legs",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    append(&route_b, "legs", Value::Object(leg.clone()));

    em.persist(&route_a).unwrap();
    em.persist(&route_b).unwrap();

    // exception depending on the underlying Database Driver
    let e = em.flush().await.unwrap_err();
    assert!(
        e.class.starts_with("Doctrine\\DBAL\\Exception"),
        "a DBAL exception: {e:?}"
    );
}
