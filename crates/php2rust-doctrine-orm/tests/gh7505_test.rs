//! Ticket/GH7505Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7505Test.php

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

use php_value::{PhpArray, Value};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testSimpleArrayTypeHydratedCorrectly() {
    let em = ticket::set_up(&[
        "GH7505AbstractResponse",
        "GH7505ArrayResponse",
        "GH7505TextResponse",
    ])
    .await;
    let array_response = new("GH7505ArrayResponse");
    em.persist(&array_response).unwrap();

    let text_response = new("GH7505TextResponse");
    em.persist(&text_response).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let repository = em.get_repository(&class("GH7505AbstractResponse")).unwrap();

    let array_response = repository
        .find(get(&array_response, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(array_response.instance_of(&class("GH7505ArrayResponse")));
    assert!(Value::Array(PhpArray::new()).identical(&get(&array_response, "value")));

    let text_response = repository
        .find(get(&text_response, "id"))
        .await
        .unwrap()
        .unwrap();
    assert!(text_response.instance_of(&class("GH7505TextResponse")));
    assert!(get(&text_response, "value").is_null());
}
