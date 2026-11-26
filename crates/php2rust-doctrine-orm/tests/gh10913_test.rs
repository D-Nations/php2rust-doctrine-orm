//! Ticket/GH10913Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10913Test.php

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

use std::sync::Arc;

use php_value::Object;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::internal::CycleDetectedException;

use orm::*;
use ticket::*;

async fn set_up() -> (Arc<EntityManager>, Arc<query_log::QueryLog>) {
    let classes = [class("GH10913Entity")];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    query_log::set_up_logged(&classes).await
}

fn create_entities(count: usize) -> Vec<Object> {
    (0..count).map(|_| new("GH10913Entity")).collect()
}

async fn persist_flush(em: &EntityManager, entities: &[&Object]) {
    for entity in entities {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
}

async fn remove(em: &EntityManager, entities: &[&Object]) {
    for entity in entities {
        em.remove(entity).await.unwrap();
    }
}

async fn flush_and_assert_number_of_delete_queries(
    em: &EntityManager,
    log: &query_log::QueryLog,
    expected_count: usize,
) {
    log.reset_and_enable();
    em.flush().await.unwrap();

    let queries = log
        .queries()
        .into_iter()
        .filter(|sql| sql.starts_with("DELETE"))
        .count();

    assert_eq!(expected_count, queries);
}

#[tokio::test]
async fn testExample1() {
    let (em, log) = set_up().await;
    let e = create_entities(3);
    let (a, b, c) = (&e[0], &e[1], &e[2]);

    set_obj(c, "ref", b);
    set_obj(b, "odc", a);

    persist_flush(&em, &[a, b, c]).await;

    remove(&em, &[a, b, c]).await;

    flush_and_assert_number_of_delete_queries(&em, &log, 3).await;
}

#[tokio::test]
async fn testExample2() {
    let (em, log) = set_up().await;
    let e = create_entities(3);
    let (a, b, c) = (&e[0], &e[1], &e[2]);

    set_obj(a, "odc", b);
    set_obj(b, "odc", a);
    set_obj(c, "ref", b);

    persist_flush(&em, &[a, b, c]).await;

    remove(&em, &[a, b, c]).await;

    flush_and_assert_number_of_delete_queries(&em, &log, 3).await;
}

#[tokio::test]
async fn testExample3() {
    let (em, _log) = set_up().await;
    let e = create_entities(3);
    let (a, b, c) = (&e[0], &e[1], &e[2]);

    set_obj(a, "odc", b);
    set_obj(a, "ref", c);
    set_obj(c, "ref", b);
    set_obj(b, "odc", a);

    persist_flush(&em, &[a, b, c]).await;

    remove(&em, &[a, b, c]).await;

    let error = em.flush().await.expect_err("a CycleDetectedException");
    assert_eq!(CycleDetectedException::CLASS, error.class);
}

#[tokio::test]
async fn testExample4() {
    let (em, log) = set_up().await;
    let e = create_entities(4);
    let (a, b, c, d) = (&e[0], &e[1], &e[2], &e[3]);

    set_obj(a, "ref", b);
    set_obj(b, "odc", c);
    set_obj(c, "odc", b);
    set_obj(d, "ref", c);

    persist_flush(&em, &[a, b, c, d]).await;

    remove(&em, &[b, c, d, a]).await;

    flush_and_assert_number_of_delete_queries(&em, &log, 4).await;
}
