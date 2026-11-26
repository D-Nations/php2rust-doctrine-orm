//! Ticket/DDC444Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC444Test.php

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
async fn testExplicitPolicy() {
    let em = ticket::set_up(&["DDC444User"]).await;
    let classname = class("DDC444User");

    let u = new("DDC444User");
    set(&u, "name", s("Initial value"));

    em.persist(&u).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(&format!("SELECT u FROM {classname} u"));
    let u = obj(&q.get_single_result(None).await.unwrap());
    assert_eq!(s("Initial value"), get(&u, "name"));

    set(&u, "name", s("Modified value"));

    // This should be NOOP as the change hasn't been persisted
    em.flush().await.unwrap();
    em.clear().unwrap();

    let u = obj(&q.get_single_result(None).await.unwrap());

    assert_eq!(s("Initial value"), get(&u, "name"));

    set(&u, "name", s("Modified value"));
    em.persist(&u).unwrap();
    // Now we however persisted it, and this should have updated our friend
    em.flush().await.unwrap();

    let q = em.create_query(&format!("SELECT u FROM {classname} u"));
    let u = obj(&q.get_single_result(None).await.unwrap());

    assert_eq!(s("Modified value"), get(&u, "name"));
}
