//! Ticket/DDC2895Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `setLastModifiedPreUpdate()`'s body is declared with
//! `register_method()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2895Test.php

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
use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php2rust_doctrine_orm::object::register_method;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testPostLoadOneToManyInheritance() {
    let em = ticket::set_up(&["DDC2895"]).await;
    // AbstractDDC2895::setLastModifiedPreUpdate():
    // $this->setLastModified(new DateTime());
    register_method(
        &class("AbstractDDC2895"),
        "setLastModifiedPreUpdate",
        Arc::new(|entity, _| {
            set(
                entity,
                "lastModified",
                DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
            );
            Ok(())
        }),
    );

    let cm = em.get_class_metadata(&class("DDC2895")).unwrap();
    let callbacks: Vec<(String, Vec<String>)> = cm
        .read()
        .lifecycle_callbacks
        .iter()
        .map(|(event, callbacks)| (event.to_owned(), callbacks.clone()))
        .collect();
    assert_eq!(
        vec![
            (
                "prePersist".to_owned(),
                vec!["setLastModifiedPreUpdate".to_owned()]
            ),
            (
                "preUpdate".to_owned(),
                vec!["setLastModifiedPreUpdate".to_owned()]
            ),
        ],
        callbacks
    );

    let ddc2895 = new("DDC2895");
    em.persist(&ddc2895).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let ddc2895 = find(&em, "DDC2895", get(&ddc2895, "id")).await.unwrap();

    assert_ne!(Value::Null, get(&ddc2895, "lastModified"));
}
