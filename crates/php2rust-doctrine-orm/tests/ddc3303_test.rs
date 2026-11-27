//! Ticket/DDC3303Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3303Test.php

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

use php_value::Value;

use orm::{assert_loose_eq, s, set};
use ticket::*;

/// When using an embedded field in an inheritance, private properties should also be inherited.
#[tokio::test]
async fn testEmbeddedObjectsAreAlsoInherited() {
    let em = set_up(&["DDC3303Employee"]).await;

    let address = new("DDC3303Address");
    set(&address, "street", s("Somewhere"));
    set(&address, "number", Value::Int(123));
    set(&address, "city", s("Over the rainbow"));
    let employee = new("DDC3303Employee");
    set(&employee, "name", s("John Doe"));
    set(&employee, "address", Value::Object(address));
    set(&employee, "company", s("Doctrine Inc"));

    em.persist(&employee).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = find(&em, "DDC3303Employee", s("John Doe")).await.unwrap();
    assert_loose_eq(&Value::Object(employee), &Value::Object(found));
}
