//! Ticket/GH7180Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with the foreign keys enforced as
//! upstream's `EnableForeignKeys` middleware does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7180Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    set_up_with_foreign_keys(&[
        "GH7180A", "GH7180B", "GH7180C", "GH7180D", "GH7180E", "GH7180F", "GH7180G",
    ])
    .await
}

async fn persist_flush(em: &EntityManager, entities: &[&Object]) {
    for entity in entities {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
}

fn assert_is_int(entity: &Object) {
    assert!(matches!(get(entity, "id"), Value::Int(_)));
}

#[tokio::test]
async fn testIssue() {
    let em = set_up().await;
    let a = new("GH7180A");
    let b = new("GH7180B");
    let c = new("GH7180C");

    set_obj(&a, "b", &b);
    set_obj(&b, "a", &a);
    set_obj(&c, "a", &a);

    persist_flush(&em, &[&a, &b, &c]).await;

    for entity in [&a, &b, &c] {
        assert_is_int(entity);
    }
}

#[tokio::test]
async fn testIssue3NodeCycle() {
    let em = set_up().await;
    let d = new("GH7180D");
    let e = new("GH7180E");
    let f = new("GH7180F");
    let g = new("GH7180G");

    set_obj(&d, "e", &e);
    set_obj(&e, "f", &f);
    set_obj(&f, "d", &d);
    set_obj(&g, "d", &d);

    persist_flush(&em, &[&d, &e, &f, &g]).await;

    for entity in [&d, &e, &f, &g] {
        assert_is_int(entity);
    }
}
