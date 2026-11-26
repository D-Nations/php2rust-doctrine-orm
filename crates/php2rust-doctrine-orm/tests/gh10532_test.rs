//! Ticket/GH10532Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `tearDown()`'s DELETE statements are left out: every test has a
//! fresh database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10532Test.php

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

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    ticket::set_up(&["GH10532A", "GH10532B", "GH10532C", "GH10532X"]).await
}

#[tokio::test]
async fn testInserts() {
    let em = set_up().await;

    // Dependencies are $a1 -> $b -> $a2 -> $c
    let a1 = new("GH10532A");
    let b = new("GH10532B");
    let a2 = new("GH10532A");
    let c = new("GH10532C");

    set_obj(&a1, "x", &b);
    set_obj(&b, "a", &a2);
    set_obj(&a2, "x", &c);

    em.persist(&a1).unwrap();
    em.persist(&a2).unwrap();
    em.persist(&b).unwrap();
    em.persist(&c).unwrap();
    em.flush().await.unwrap();

    for entity in [&a1, &b, &a2, &c] {
        assert!(!matches!(get(entity, "id"), Value::Null));
    }
}

#[tokio::test]
async fn testDeletes() {
    let em = set_up().await;

    // Dependencies are $a1 -> $b -> $a2 -> $c

    // The "c" entity
    insert(
        &em,
        "gh10532_x",
        &[("id", Value::Int(1)), ("discr", s("C"))],
    )
    .await;
    insert(&em, "gh10532_c", &[("id", Value::Int(1))]).await;
    let c = find(&em, "GH10532C", Value::Int(1)).await.unwrap();

    // The "a2" entity
    insert(
        &em,
        "gh10532_a",
        &[("id", Value::Int(2)), ("gh10532x_id", Value::Int(1))],
    )
    .await;
    let a2 = find(&em, "GH10532A", Value::Int(2)).await.unwrap();

    // The "b" entity
    insert(
        &em,
        "gh10532_x",
        &[("id", Value::Int(3)), ("discr", s("B"))],
    )
    .await;
    insert(
        &em,
        "gh10532_b",
        &[("id", Value::Int(3)), ("gh10532a_id", Value::Int(2))],
    )
    .await;
    let b = find(&em, "GH10532B", Value::Int(3)).await.unwrap();

    // The "a1" entity
    insert(
        &em,
        "gh10532_a",
        &[("id", Value::Int(4)), ("gh10532x_id", Value::Int(3))],
    )
    .await;
    let a1 = find(&em, "GH10532A", Value::Int(4)).await.unwrap();

    em.remove(&a1).await.unwrap();
    em.remove(&a2).await.unwrap();
    em.remove(&b).await.unwrap();
    em.remove(&c).await.unwrap();
    em.flush().await.unwrap();
}
