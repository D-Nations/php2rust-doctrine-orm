//! Ticket/GH11341Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! The data providers' rows run in one test each, every row on a new
//! database. The regular expression `/WHERE [a-z]0_.type IN (...)$/` is
//! checked as the end of the SQL.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11341Test.php

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

use std::sync::Arc;

use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::exec::SqlStatements;

use ticket::*;

/// `setUp()`.
async fn set_up_schema() -> Arc<EntityManager> {
    set_up(&[
        "IntegerBaseClass",
        "IntegerFooEntity",
        "IntegerBarEntity",
        "StringAsIntBaseClass",
        "StringAsIntFooEntity",
        "StringAsIntBarEntity",
        "StringBaseClass",
        "StringFooEntity",
        "StringBarEntity",
    ])
    .await
}

/// `dqlStatements()`: the class and the values of `IN (...)`.
const DQL_STATEMENTS: [(&str, &str); 9] = [
    ("IntegerBaseClass", "1, 2"),
    ("IntegerFooEntity", "1"),
    ("IntegerBarEntity", "2"),
    ("StringAsIntBaseClass", "'1', '2'"),
    ("StringAsIntFooEntity", "'1'"),
    ("StringAsIntBarEntity", "'2'"),
    ("StringBaseClass", "'1', '2'"),
    ("StringFooEntity", "'1'"),
    ("StringBarEntity", "'2'"),
];

/// `/WHERE [a-z]0_.type IN \(...\)$/`.
fn matches_discriminator_values(sql: &str, values: &str) -> bool {
    let tail = format!("0_.type IN ({values})");
    let Some(head) = sql.strip_suffix(&tail) else {
        return false;
    };
    let mut chars = head.chars();
    chars.next_back().is_some_and(|c| c.is_ascii_lowercase())
        && chars.as_str().ends_with("WHERE ")
}

#[tokio::test]
async fn testDiscriminatorValue() {
    for (short, values) in DQL_STATEMENTS {
        let em = set_up_schema().await;
        let dql = format!("SELECT e FROM {} e", class(short));
        let query = em.create_query(&dql);
        let SqlStatements::One(sql) = query.get_sql().unwrap() else {
            panic!("one statement");
        };

        assert!(matches_discriminator_values(&sql, values), "{sql}");
    }
}

/// `dqlStatementsForInstanceOf()`.
const DQL_STATEMENTS_FOR_INSTANCE_OF: [(&str, &str); 3] = [
    ("IntegerBaseClass", "IntegerFooEntity"),
    ("StringBaseClass", "StringFooEntity"),
    ("StringAsIntBaseClass", "StringAsIntFooEntity"),
];

#[tokio::test]
async fn testInstanceOf() {
    for (base_class, inherited_class) in DQL_STATEMENTS_FOR_INSTANCE_OF {
        let em = set_up_schema().await;
        em.persist(&new(inherited_class)).unwrap();
        em.flush().await.unwrap();

        let base_class_name = class(base_class);
        let dql =
            format!("SELECT p FROM {base_class_name} p WHERE p INSTANCE OF {base_class_name}");

        let query = em.create_query(&dql);
        let result = orm::list(&query.get_object_result().await.unwrap());

        assert_eq!(1, result.len());
        for entity in &result {
            assert_instance_of(base_class, entity);
        }
    }
}
