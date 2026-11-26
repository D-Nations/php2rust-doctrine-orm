//! Ticket/DDC2175Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2175Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC2175Entity"]).await;

    let entity = new("DDC2175Entity");
    set(&entity, "field", s("foo"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(1), &get(&entity, "version"));

    set(&entity, "field", s("bar"));
    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(2), &get(&entity, "version"));

    set(&entity, "field", s("baz"));
    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(3), &get(&entity, "version"));
}
