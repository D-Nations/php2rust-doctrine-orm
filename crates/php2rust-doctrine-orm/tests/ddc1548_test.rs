//! Ticket/DDC1548Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1548Test.php

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
    let em = ticket::set_up(&["DDC1548E1", "DDC1548E2", "DDC1548Rel"]).await;

    let rel = new("DDC1548Rel");
    em.persist(&rel).unwrap();
    em.flush().await.unwrap();

    let e1 = new("DDC1548E1");
    set_obj(&e1, "rel", &rel);
    em.persist(&e1).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let obt = find(&em, "DDC1548Rel", get(&rel, "id")).await.unwrap();

    assert_eq!(Value::Null, get(&obt, "e2"));
}
