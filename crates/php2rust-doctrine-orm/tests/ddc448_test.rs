//! Ticket/DDC448Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC448Test.php

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
    let em = ticket::set_up(&["DDC448MainTable", "DDC448ConnectedClass", "DDC448SubTable"]).await;

    let q = em.create_query(&format!(
        "select b from {NS}DDC448SubTable b where b.connectedClassId = ?1"
    ));
    let SqlStatements::One(sql) = q.get_sql().unwrap() else {
        panic!("one statement");
    };
    assert_eq!(
        "SELECT d0_.id AS id_0, d0_.discr AS discr_1, d0_.connectedClassId AS connectedClassId_2 FROM SubTable s1_ INNER JOIN DDC448MainTable d0_ ON s1_.id = d0_.id WHERE d0_.connectedClassId = ?".to_lowercase(),
        sql.to_lowercase(),
    );
}
