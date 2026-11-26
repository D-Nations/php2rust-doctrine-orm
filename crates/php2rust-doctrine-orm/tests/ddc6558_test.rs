//! Ticket/DDC6558Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC6558Test.php

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
async fn testEmployeeIsPopulated() {
    let em = ticket::set_up(&[
        "DDC6558Person",
        "DDC6558Employee",
        "DDC6558Staff",
        "DDC6558Developer",
        "DDC6558Manager",
    ])
    .await;

    let developer = new("DDC6558Developer");
    set(&developer, "phoneNumber", Value::Int(1_231_231_231));
    set(&developer, "emailAddress", s("email@address.com"));

    em.persist(&developer).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let persisted_developer = find(&em, "DDC6558Person", get(&developer, "id"))
        .await
        .unwrap();

    assert!(!matches!(
        get(&persisted_developer, "phoneNumber"),
        Value::Null
    ));
    assert!(!matches!(
        get(&persisted_developer, "emailAddress"),
        Value::Null
    ));
}
