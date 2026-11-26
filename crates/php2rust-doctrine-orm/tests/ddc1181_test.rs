//! Ticket/DDC1181Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1181Test.php

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

use php_value::Value;

use orm::*;

const HOTEL: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1181Hotel";
const BOOKING: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1181Booking";
const ROOM: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1181Room";

#[tokio::test]
async fn testIssue() {
    let em = orm::set_up(&[HOTEL, BOOKING, ROOM]).await;

    let hotel = new_object(HOTEL);
    let room1 = new_object(ROOM);
    let room2 = new_object(ROOM);

    em.persist(&hotel).unwrap();
    em.persist(&room1).unwrap();
    em.persist(&room2).unwrap();
    em.flush().await.unwrap();

    let booking1 = new_object(BOOKING);
    set_obj(&booking1, "hotel", &hotel);
    set_obj(&booking1, "room", &room1);
    let booking2 = new_object(BOOKING);
    set_obj(&booking2, "hotel", &hotel);
    set_obj(&booking2, "room", &room2);
    append(&hotel, "bookings", Value::Object(booking1.clone()));
    append(&hotel, "bookings", Value::Object(booking2.clone()));

    em.persist(&booking1).unwrap();
    em.persist(&booking2).unwrap();
    em.flush().await.unwrap();

    em.remove(&hotel).await.unwrap();
    em.flush().await.unwrap();

    assert!(
        em.get_repository(BOOKING)
            .unwrap()
            .find_all()
            .await
            .unwrap()
            .is_empty()
    );
}
