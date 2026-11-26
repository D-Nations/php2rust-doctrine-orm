//! Ticket/GH8663Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8663Test.php

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
use php2rust_doctrine_dbal::LockMode;

use ticket::*;

#[tokio::test]
async fn testDeletedEntity() {
    let em = ticket::set_up(&["GH8663VersionedEntity"]).await;
    let result = em
        .find_with_lock(
            &class("GH8663VersionedEntity"),
            Value::Int(1),
            LockMode::Optimistic,
            None,
        )
        .await
        .unwrap();

    assert!(result.is_none());
}
