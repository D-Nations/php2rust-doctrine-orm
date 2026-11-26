//! Ticket/GH7259Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with the foreign keys enforced as
//! upstream's `EnableForeignKeys` middleware does.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7259Test.php

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

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    set_up_with_foreign_keys(&[
        "GH7259Space",
        "GH7259File",
        "GH7259FileVersion",
        "GH7259Feed",
    ])
    .await
}

#[tokio::test]
async fn testPersistFileBeforeVersion() {
    let em = set_up().await;
    let space = new("GH7259Space");

    em.persist(&space).unwrap();
    em.flush().await.unwrap();

    let feed = new("GH7259Feed");
    set_obj(&feed, "space", &space);

    let file = new("GH7259File");
    set_obj(&file, "space", &space);
    let file_version = new("GH7259FileVersion");
    set_obj(&file_version, "file", &file);

    em.persist(&file).unwrap();
    em.persist(&file_version).unwrap();
    em.persist(&feed).unwrap();

    em.flush().await.unwrap();

    assert!(!get(&file_version, "id").is_null());
}

#[tokio::test]
async fn testPersistFileAfterVersion() {
    let em = set_up().await;
    let space = new("GH7259Space");

    em.persist(&space).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let space = find(&em, "GH7259Space", get(&space, "id")).await.unwrap();

    let feed = new("GH7259Feed");
    set_obj(&feed, "space", &space);

    let file = new("GH7259File");
    set_obj(&file, "space", &space);
    let file_version = new("GH7259FileVersion");
    set_obj(&file_version, "file", &file);

    em.persist(&file_version).unwrap();
    em.persist(&file).unwrap();
    em.persist(&feed).unwrap();

    em.flush().await.unwrap();

    assert!(!get(&file_version, "id").is_null());
}
