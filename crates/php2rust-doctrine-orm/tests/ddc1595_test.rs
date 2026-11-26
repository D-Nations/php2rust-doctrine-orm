//! Ticket/DDC1595Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `getLastLoggedQuery()` is the last statement of the query log.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1595Test.php

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/ticket.rs"]
mod ticket;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let classes = [
        class("DDC1595BaseInheritance"),
        class("DDC1595InheritedEntity1"),
        class("DDC1595InheritedEntity2"),
    ];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;
    // The functional test case logs every statement.
    log.reset_and_enable();
    let last_logged_query = || log.queries().last().cloned().unwrap_or_default();
    // assertSQLEquals() compares lower case.
    let assert_sql_equals = |expected: &str, actual: String| {
        assert_eq!(expected.to_lowercase(), actual.to_lowercase());
    };

    let e1 = new("DDC1595InheritedEntity1");

    em.persist(&e1).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let repository = em
        .get_repository(&class("DDC1595InheritedEntity1"))
        .unwrap();

    let entity1 = repository.find(get(&e1, "id")).await.unwrap().unwrap();

    assert_sql_equals(
        "SELECT t0.id AS id_1, t0.type FROM base t0 WHERE t0.id = ? AND t0.type IN ('Entity1')",
        last_logged_query(),
    );

    elements(&entity1, "entities").await;
    assert_eq!(
        "SELECT t0.id AS id_1, t0.type FROM base t0 INNER JOIN entity1_entity2 ON t0.id = entity1_entity2.item WHERE entity1_entity2.parent = ? AND t0.type IN ('Entity2')",
        last_logged_query()
    );

    em.clear().unwrap();

    let entity1 = repository.find(get(&e1, "id")).await.unwrap().unwrap();
    count_of(&entity1, "entities").await;

    assert_sql_equals(
        "SELECT COUNT(*) FROM entity1_entity2 t WHERE t.parent = ?",
        last_logged_query(),
    );
}
