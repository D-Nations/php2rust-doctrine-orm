//! Ticket/DDC1458Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! A method call on a lazy proxy initializes it: the test calls
//! `initialize_object()` where PHP calls `getBool()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1458Test.php

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

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["TestEntity", "TestAdditionalEntity"]).await;

    let test_entity = new("TestEntity");
    set(&test_entity, "value", Value::Int(3));
    let additional = new("TestAdditionalEntity");
    set(&additional, "bool", Value::Bool(false));
    set_obj(&test_entity, "additional", &additional);

    em.persist(&test_entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_loose_eq(&Value::Int(3), &get(&test_entity, "value"));

    let test = em
        .get_repository(&class("TestEntity"))
        .unwrap()
        .find(Value::Int(1))
        .await
        .unwrap()
        .unwrap();
    set(&test, "value", Value::Int(5));
    assert_loose_eq(&Value::Int(5), &get(&test, "value"));

    let additional = get_object(&test, "additional").unwrap();
    assert_loose_eq(&Value::Int(5), &get(&test, "value"));

    // $additional->getBool()
    em.initialize_object(&additional).await.unwrap();
    let _ = get(&additional, "bool");
    assert_loose_eq(&Value::Int(5), &get(&test, "value"));
}
