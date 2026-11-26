//! Ticket/GH7012Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the quote models and its own model).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7012Test.php

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
#[path = "common/quote.rs"]
mod quote;
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{PhpArray, Value};

use orm::*;
use quote::*;

#[tokio::test]
async fn testUpdateEntityWithIdentifierAssociationWithQuotedJoinColumn() {
    let user_data_class = ticket::class("GH7012UserData");
    let mut classes = QUOTE_MODELS.to_vec();
    classes.push(&user_data_class);
    let em = orm::set_up(&classes).await;

    let user = quote_user("John Doe");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    // new GH7012UserData($user, '123456789')
    let user_data = ticket::new("GH7012UserData");
    set_obj(&user_data, "user", &user);
    set(&user_data, "name", s("123456789"));

    em.persist(&user_data).unwrap();
    em.flush().await.unwrap();

    set(&user_data, "name", s("4321"));
    em.flush().await.unwrap();

    let mut criteria = PhpArray::new();
    criteria.insert("user", Value::Object(user.clone()));
    let found = em
        .get_repository(&user_data_class)
        .unwrap()
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .unwrap();
    assert!(s("4321").identical(&get(&found, "name")));
}
