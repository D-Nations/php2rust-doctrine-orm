//! Ticket/DDC1695Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models, no database).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1695Test.php

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

use php2rust_doctrine_orm::query::exec::SqlStatements;

use ticket::*;

#[tokio::test]
async fn testIssue() {
    // Only with sqlite: the port's connection is SQLite.
    let em = orm::entity_manager(orm::connection(), None);
    let dql = format!(
        "SELECT n.smallText, n.publishDate FROM {}",
        class("DDC1695News")
    ) + " n";
    let SqlStatements::One(sql) = em.create_query(&dql).get_sql().unwrap() else {
        panic!("one statement")
    };

    assert_eq!(
        "SELECT d0_.\"SmallText\" AS SmallText_0, d0_.\"PublishDate\" AS PublishDate_1 FROM \"DDC1695News\" d0_",
        sql
    );
}
