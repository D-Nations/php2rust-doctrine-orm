//! Ticket/DDC1515Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1515Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC1515Foo", "DDC1515Bar"]).await;

    let bar = new("DDC1515Bar");
    em.persist(&bar).unwrap();
    em.flush().await.unwrap();

    let foo = new("DDC1515Foo");
    set_obj(&foo, "bar", &bar);
    em.persist(&foo).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let bar = find(&em, "DDC1515Bar", get(&bar, "id")).await.unwrap();
    assert_instance_of("DDC1515Foo", &get(&bar, "foo"));
}
