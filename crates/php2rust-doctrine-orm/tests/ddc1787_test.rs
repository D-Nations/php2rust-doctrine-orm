//! Ticket/DDC1787Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1787Test.php

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
    let em = ticket::set_up(&["DDC1787Foo", "DDC1787Bar"]).await;

    let bar = new("DDC1787Bar");
    let bar2 = new("DDC1787Bar");

    em.persist(&bar).unwrap();
    em.persist(&bar2).unwrap();
    em.flush().await.unwrap();

    assert_eq!(Value::Int(1), get(&bar, "version"));
}
