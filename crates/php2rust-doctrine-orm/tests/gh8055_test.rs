//! Ticket/GH8055Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8055Test.php

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
async fn testNumericDescriminatorColumn() {
    let em = ticket::set_up(&["GH8055BaseClass", "GH8055SubClass"]).await;
    let entity = new("GH8055SubClass");
    set(&entity, "value", s("test"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let repository = em.get_repository(&class("GH8055SubClass")).unwrap();
    let hydrated = repository
        .find(get(&entity, "id"))
        .await
        .unwrap()
        .unwrap();

    assert!(s("test").identical(&get(&hydrated, "value")));
}
