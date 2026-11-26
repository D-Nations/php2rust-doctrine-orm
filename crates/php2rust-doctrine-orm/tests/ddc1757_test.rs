//! Ticket/DDC1757Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1757Test.php

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
async fn testFailingCase() {
    let em = orm::set_up(&[]).await;

    let mut qb = em.create_query_builder();
    qb.select(vec![s("_a")])
        .unwrap()
        .from(&class("DDC1757A"), "_a", None)
        .unwrap()
        .from(&class("DDC1757B"), "_b", None)
        .unwrap()
        .join("_b.c", "_c", None, None, None)
        .unwrap()
        .join("_c.d", "_d", None, None, None)
        .unwrap();

    let q = qb.get_query().unwrap();
    let dql = q.get_dql().unwrap();
    assert_eq!(
        format!(
            "SELECT _a FROM {NS}DDC1757A _a, {NS}DDC1757B _b INNER JOIN _b.c _c INNER JOIN _c.d _d"
        ),
        dql,
        "Wrong DQL query"
    );
}
