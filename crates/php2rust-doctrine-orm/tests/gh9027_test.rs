//! Ticket/GH9027Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; the hints array is the port's `Hints`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9027Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EAGER;
use php2rust_doctrine_orm::unit_of_work::{Hints, UnitOfWork};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testUnitOfWorkHandlesNullRelations() {
    let em = ticket::set_up(&["GH9027Cart", "GH9027Customer"]).await;
    let uow = UnitOfWork::new(
        Arc::downgrade(&em),
        em.get_configuration().get_entity_listener_resolver(),
    );
    let mut hints = Hints {
        fetch_mode: vec![(class("GH9027Cart"), "customer".to_owned(), Some(FETCH_EAGER))],
        ..Hints::default()
    };

    let mut data = PhpArray::new();
    data.insert("id", Value::Int(1));
    data.insert("customer", Value::Int(24252));
    let cart = uow
        .create_entity(&class("GH9027Cart"), data, &mut hints)
        .await
        .unwrap();

    assert_loose_eq(&Value::Null, &get(&cart, "customer"));
}
