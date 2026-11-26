//! Ticket/GH11199Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! The data provider's rows run in one test; each regular expression is
//! a suffix (`...$`) or a substring of the SQL.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11199Test.php

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

/// `dqlStatements()`: the DQL and the end (`true`) or a part (`false`)
/// of its SQL.
fn dql_statements() -> Vec<(String, &'static str, bool)> {
    vec![
        (
            format!("SELECT e FROM {} e", class("GH11199Root")),
            "WHERE g0_.asset_type IN ('root', 'foo', 'baz')",
            true,
        ),
        (
            format!("SELECT e FROM {} e", class("GH11199Parent")),
            "WHERE g0_.asset_type IN ('foo')",
            true,
        ),
        (
            format!("SELECT e FROM {} e", class("GH11199Foo")),
            "WHERE g0_.asset_type IN ('foo')",
            true,
        ),
        (
            format!("SELECT e FROM {} e", class("GH11199Baz")),
            "WHERE g0_.asset_type IN ('baz')",
            true,
        ),
        (
            format!("SELECT e FROM {} e", class("GH11199AbstractLeaf")),
            "WHERE 1=0",
            false,
        ),
    ]
}

#[tokio::test]
async fn testGH11199() {
    let em = set_up(&[
        "GH11199Root",
        "GH11199Parent",
        "GH11199Foo",
        "GH11199Baz",
        "GH11199AbstractLeaf",
    ])
    .await;

    for (dql, expected, at_end) in dql_statements() {
        let query = em.create_query(&dql);
        let SqlStatements::One(sql) = query.get_sql().unwrap() else {
            panic!("one statement");
        };

        if at_end {
            assert!(sql.ends_with(expected), "{sql}");
        } else {
            assert!(sql.contains(expected), "{sql}");
        }
    }
}
