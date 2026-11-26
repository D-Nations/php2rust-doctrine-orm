//! Ticket/GH7006Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7006Test.php

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
    let em = ticket::set_up(&["GH7006Book", "GH7006PCT", "GH7006PCTFee"]).await;

    let book = new("GH7006Book");
    set(&book, "exchangeCode", s("first"));
    em.persist(&book).unwrap();

    set(&book, "exchangeCode", s("second")); // change sth.

    // new GH7006PCT()
    let payment_card_transaction = new("GH7006PCT");
    set(
        &payment_card_transaction,
        "fees",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set_obj(&payment_card_transaction, "book", &book);
    // new GH7006PCTFee($paymentCardTransaction)
    let payment_card_transaction_fee = new("GH7006PCTFee");
    set_obj(
        &payment_card_transaction_fee,
        "pct",
        &payment_card_transaction,
    );
    coll::add(
        &get(&payment_card_transaction, "fees"),
        Value::Object(payment_card_transaction_fee.clone()),
    )
    .unwrap();

    em.persist(&payment_card_transaction).unwrap();

    em.flush().await.unwrap();

    assert!(matches!(get(&book, "id"), Value::Int(_)));
    assert!(matches!(get(&payment_card_transaction, "id"), Value::Int(_)));
    assert!(matches!(
        get(&payment_card_transaction_fee, "id"),
        Value::Int(_)
    ));
}
