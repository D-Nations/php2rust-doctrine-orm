//! Ticket/DDC3042Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3042Test.php

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
async fn testSQLGenerationDoesNotProvokeAliasCollisions() {
    let em = ticket::set_up(&["DDC3042Foo", "DDC3042Bar"]).await;

    let SqlStatements::One(sql) = em
        .create_query(&format!(
            "SELECT f, b FROM {NS}DDC3042Foo f JOIN {NS}DDC3042Bar b ON 1 = 1"
        ))
        .get_sql()
        .unwrap()
    else {
        panic!("one statement");
    };

    assert_eq!(
        1,
        sql.matches("field_11").count(),
        "The alias \"field11\" should only appear once in the SQL query."
    );
}
