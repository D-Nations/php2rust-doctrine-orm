//! Ticket/GH7068Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7068Test.php

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
use php2rust_doctrine_dbal::LockMode;

use ticket::*;

#[tokio::test]
async fn testLockModeIsRespected() {
    let em = ticket::set_up(&["SomeEntity"]).await;
    let entity = new("SomeEntity");
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    em.find(&class("SomeEntity"), Value::Int(1)).await.unwrap();

    let e = em
        .find_with_lock(
            &class("SomeEntity"),
            Value::Int(1),
            LockMode::PessimisticWrite,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(
        "Doctrine\\ORM\\TransactionRequiredException", e.class,
        "{e:?}"
    );
}
