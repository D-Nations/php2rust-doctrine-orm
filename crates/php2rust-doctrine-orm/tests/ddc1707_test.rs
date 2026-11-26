//! Ticket/DDC1707Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `onPostLoad()`'s body is declared with `register_method()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1707Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::object::register_method;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testPostLoadOnChild() {
    let em = ticket::set_up(&["DDC1707Base", "DDC1707Child"]).await;
    // DDC1707Base::onPostLoad(): $this->postLoad = true;
    register_method(
        &class("DDC1707Base"),
        "onPostLoad",
        Arc::new(|entity, _| {
            set(entity, "postLoad", Value::Bool(true));
            Ok(())
        }),
    );

    let class = em.get_class_metadata(&class("DDC1707Child")).unwrap();
    let entity = new("DDC1707Child");
    class
        .read()
        .invoke_lifecycle_callbacks(events::POST_LOAD, &entity)
        .unwrap();

    assert_eq!(Value::Bool(true), get(&entity, "postLoad"));
}
