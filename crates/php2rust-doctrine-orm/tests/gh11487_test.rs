//! Ticket/GH11487Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11487Test.php

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

#[tokio::test]
async fn testItThrowsASyntaxErrorOnUnfinishedQuery() {
    let em = orm::set_up(&[]).await;

    let e = em
        .create_query("UPDATE Doctrine\\Tests\\ORM\\Functional\\Ticket\\TaxType t SET t.default =")
        .execute(None, None)
        .await
        .unwrap_err();

    assert_eq!("Doctrine\\ORM\\Query\\QueryException", e.class, "{e:?}");
    assert!(orm::message(&e).contains("Syntax Error"), "{e:?}");
}
