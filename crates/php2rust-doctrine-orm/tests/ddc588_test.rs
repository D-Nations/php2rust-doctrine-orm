//! Ticket/DDC588Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC588Test.php

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

use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC588Site"]).await;

    let site = new("DDC588Site");
    orm::set(&site, "name", orm::s("Foo"));
    em.persist(&site).unwrap();
    em.flush().await.unwrap();
    // Following should not result in exception
    em.refresh(&site).await.unwrap();
}
