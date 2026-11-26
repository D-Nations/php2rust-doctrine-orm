//! Ticket/DDC1454Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `random_int(0, mt_getrandmax())` in the constructor is a fixed id.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1454Test.php

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
use php2rust_doctrine_orm::unit_of_work::STATE_NEW;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testFailingCase() {
    let em = ticket::set_up(&["DDC1454File", "DDC1454Picture"]).await;

    let pic = new("DDC1454Picture");
    set(&pic, "fileId", Value::Int(1_804_289_383));

    assert_eq!(
        STATE_NEW,
        em.get_unit_of_work().get_entity_state(&pic).await.unwrap()
    );
}
