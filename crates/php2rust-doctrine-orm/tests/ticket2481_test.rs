//! Ticket/Ticket2481Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/Ticket2481Test.php

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
async fn testEmptyInsert() {
    let em = ticket::set_up(&["Ticket2481Product"]).await;
    let test = new("Ticket2481Product");
    em.persist(&test).unwrap();
    em.flush().await.unwrap();

    assert!(matches!(get(&test, "id"), Value::Int(id) if id > 0));
}
