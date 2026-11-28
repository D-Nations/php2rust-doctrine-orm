//! Ticket/GH6464Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! The regular expression `/INNER JOIN \w+ \w+ INNER JOIN/` is matched by
//! hand.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6464Test.php

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

use orm::*;
use ticket::*;

/// `\w+ ` at the start of `text`: the rest after the space.
fn word_then_space(text: &str) -> Option<&str> {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len());
    if end == 0 {
        return None;
    }
    text[end..].strip_prefix(' ')
}

/// `preg_match('/INNER JOIN \w+ \w+ INNER JOIN/', $sql)`.
fn has_inner_join_without_on(sql: &str) -> bool {
    sql.match_indices("INNER JOIN ").any(|(at, needle)| {
        word_then_space(&sql[at + needle.len()..])
            .and_then(word_then_space)
            .is_some_and(|rest| rest.starts_with("INNER JOIN"))
    })
}

/// Verifies that `SqlWalker` generates valid SQL for an INNER JOIN to CTI table
///
/// `SqlWalker` needs to generate nested INNER JOIN statements, otherwise there would be INNER JOIN
/// statements without an ON clause, which are valid on e.g. `MySQL` but rejected by `PostgreSQL`.
#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["GH6464Post", "GH6464User", "GH6464Author"]).await;

    let mut qb = em.create_query_builder();
    qb.select(vec![s("p")])
        .unwrap()
        .from(&class("GH6464Post"), "p", None)
        .unwrap()
        .inner_join(
            &class("GH6464Author"),
            "a",
            Some("ON"),
            Some(s("p.authorId = a.id")),
            None,
        )
        .unwrap();
    let query = qb.get_query().unwrap();

    let SqlStatements::One(sql) = query.get_sql().unwrap() else {
        panic!("one statement");
    };
    assert!(
        !has_inner_join_without_on(&sql),
        "As of GH-6464, every INNER JOIN should have an ON clause, which is missing here: {sql}"
    );

    // Query shouldn't yield a result, yet it shouldn't crash (anymore)
    assert_eq!(list(&query.get_object_result().await.unwrap()).len(), 0);
}
