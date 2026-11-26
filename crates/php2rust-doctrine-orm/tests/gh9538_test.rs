//! Ticket/GH9538Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9538Test.php

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
async fn testCanRemoveEntityWithReadonlyId() {
    let em = ticket::set_up(&["GH9538EntityA"]).await;
    let entity = new("GH9538EntityA");
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    em.remove(&entity).await.unwrap();
    em.flush().await.unwrap();
}
