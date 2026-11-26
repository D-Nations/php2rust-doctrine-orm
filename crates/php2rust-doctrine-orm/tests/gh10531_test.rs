//! Ticket/GH10531Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `tearDown()`'s DELETE statements are left out: every test has a
//! fresh database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10531Test.php

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
    ticket::set_up(&["GH10531A", "GH10531B"]).await
}

#[tokio::test]
async fn testInserts() {
    let em = set_up().await;

    let a = new("GH10531A");
    let b1 = new("GH10531B");
    let b2 = new("GH10531B");
    let b3 = new("GH10531B");

    set_obj(&b1, "parent", &b2);
    set_obj(&b3, "parent", &b2);
    set_obj(&b2, "parent", &a);

    // Pass $b2 to persist() between $b1 and $b3, so that any potential reliance upon the
    // order of persist() calls is spotted.
    em.persist(&a).unwrap();
    em.persist(&b1).unwrap();
    em.persist(&b2).unwrap();
    em.persist(&b3).unwrap();
    em.flush().await.unwrap();

    for entity in [&a, &b1, &b2, &b3] {
        assert!(!matches!(get(entity, "id"), Value::Null));
    }
}

#[tokio::test]
async fn testDeletes() {
    let em = set_up().await;

    // The "a" entity
    insert(
        &em,
        "gh10531_a",
        &[("id", Value::Int(1)), ("discr", s("A"))],
    )
    .await;
    let _a = find(&em, "GH10531A", Value::Int(1)).await.unwrap();

    // The "b2" entity
    insert(
        &em,
        "gh10531_a",
        &[("id", Value::Int(2)), ("discr", s("B"))],
    )
    .await;
    insert(
        &em,
        "gh10531_b",
        &[("id", Value::Int(2)), ("parent_id", Value::Int(1))],
    )
    .await;
    let b2 = find(&em, "GH10531B", Value::Int(2)).await.unwrap();

    // The "b1" entity
    insert(
        &em,
        "gh10531_a",
        &[("id", Value::Int(3)), ("discr", s("B"))],
    )
    .await;
    insert(
        &em,
        "gh10531_b",
        &[("id", Value::Int(3)), ("parent_id", Value::Int(2))],
    )
    .await;
    let b1 = find(&em, "GH10531B", Value::Int(3)).await.unwrap();

    // The "b3" entity
    insert(
        &em,
        "gh10531_a",
        &[("id", Value::Int(4)), ("discr", s("B"))],
    )
    .await;
    insert(
        &em,
        "gh10531_b",
        &[("id", Value::Int(4)), ("parent_id", Value::Int(2))],
    )
    .await;
    let b3 = find(&em, "GH10531B", Value::Int(4)).await.unwrap();

    // As before, put $b2 in between $b1 and $b3 so that the order of the
    // remove() calls alone (in either direction) does not solve the problem.
    // The ORM will have to sort $b2 to be deleted last, after $b1 and $b3.
    em.remove(&b1).await.unwrap();
    em.remove(&b2).await.unwrap();
    em.remove(&b3).await.unwrap();
    em.flush().await.unwrap();
}
