//! Ticket/DDC144Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC144Test.php

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
    let em = ticket::set_up(&["DDC144FlowElement", "DDC144Operand"]).await;

    let operand = new("DDC144Operand");
    set(&operand, "property", s("flowValue"));
    set(&operand, "operandProperty", s("operandValue"));

    em.persist(&operand).unwrap();
    em.flush().await.unwrap();

    let found = find(&em, "DDC144Operand", get(&operand, "id"))
        .await
        .unwrap();
    assert!(same(&operand, &found));
}
