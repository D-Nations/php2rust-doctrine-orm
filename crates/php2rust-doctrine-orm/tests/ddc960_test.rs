//! Ticket/DDC960Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC960Test.php

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
async fn testUpdateRootVersion() {
    let em = ticket::set_up(&["DDC960Root", "DDC960Child"]).await;

    let child = new("DDC960Child");
    set(&child, "name", s("Test"));
    em.persist(&child).unwrap();
    em.flush().await.unwrap();

    set(&child, "name", s("Test2"));

    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(2), &get(&child, "version"));
}
