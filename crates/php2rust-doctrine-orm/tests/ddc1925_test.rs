//! Ticket/DDC1925Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1925Test.php

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
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC1925User", "DDC1925Product"]).await;

    let user = new("DDC1925User");
    set(&user, "title", s("Test User"));

    let product = new("DDC1925Product");
    set(
        &product,
        "buyers",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&product, "title", s("Test product"));

    em.persist(&user).unwrap();
    em.persist(&product).unwrap();
    em.flush().await.unwrap();

    // $product->addBuyer($user)
    append(&product, "buyers", Value::Object(user.clone()));

    em.get_unit_of_work().compute_change_sets().await.unwrap();

    em.persist(&product).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let persisted_product = find(&em, "DDC1925Product", get(&product, "id"))
        .await
        .unwrap();

    let first = coll::first(&get(&persisted_product, "buyers"))
        .await
        .unwrap();
    assert_loose_eq(&Value::Object(user), &first);
}
