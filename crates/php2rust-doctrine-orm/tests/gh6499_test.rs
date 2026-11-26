//! Ticket/GH6499Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with the foreign keys enforced as
//! upstream's `EnableForeignKeys` middleware does.
//!
//! Specifically, GH6499B has a dependency on GH6499A, and GH6499A
//! has a dependency on GH6499B. Since GH6499A#b is not nullable,
//! the database row for GH6499B should be inserted first.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6499Test.php

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
    let em = set_up_with_foreign_keys(&["GH6499A", "GH6499B"]).await;
    let b = new("GH6499B");
    let a = new("GH6499A");

    em.persist(&a).unwrap();

    set_obj(&a, "b", &b);

    em.persist(&b).unwrap();

    em.flush().await.unwrap();

    assert!(matches!(get(&a, "id"), Value::Int(_)));
    assert!(matches!(get(&b, "id"), Value::Int(_)));
}

#[tokio::test]
async fn testIssueReversed() {
    let em = set_up_with_foreign_keys(&["GH6499A", "GH6499B"]).await;
    let b = new("GH6499B");
    let a = new("GH6499A");

    set_obj(&a, "b", &b);

    em.persist(&b).unwrap();
    em.persist(&a).unwrap();

    em.flush().await.unwrap();

    assert!(matches!(get(&a, "id"), Value::Int(_)));
    assert!(matches!(get(&b, "id"), Value::Int(_)));
}
