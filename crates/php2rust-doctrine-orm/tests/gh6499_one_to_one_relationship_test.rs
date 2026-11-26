//! Ticket/GH6499OneToOneRelationshipTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with the foreign keys enforced as
//! upstream's `EnableForeignKeys` middleware does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6499OneToOneRelationshipTest.php

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

/// Test for the bug described in issue #6499.
#[tokio::test]
async fn testIssue() {
    let em = set_up_with_foreign_keys(&["GH6499OTOA", "GH6499OTOB"]).await;

    // new GH6499OTOA()
    let a = new("GH6499OTOA");
    set_obj(&a, "b", &new("GH6499OTOB"));

    em.persist(&a).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = find(&em, "GH6499OTOA", get(&a, "id")).await.unwrap();
    assert_loose_eq(
        &get(&obj(&get(&found, "b")), "id"),
        &get(&obj(&get(&a, "b")), "id"),
    );
}
