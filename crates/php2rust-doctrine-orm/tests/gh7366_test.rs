//! Ticket/GH7366Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7366Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testOptimisticLockNoExceptionOnFind() {
    let em = ticket::set_up(&["GH7366Entity"]).await;
    // new GH7366Entity('baz')
    let entity = new("GH7366Entity");
    set(&entity, "name", s("baz"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let entity = match em
        .find_with_lock(
            &class("GH7366Entity"),
            Value::Int(1),
            LockMode::Optimistic,
            None,
        )
        .await
    {
        Err(e) if e.class == "Doctrine\\ORM\\TransactionRequiredException" => panic!(
            "EntityManager::find() threw TransactionRequiredException with LockMode::OPTIMISTIC"
        ),
        result => result.unwrap().unwrap(),
    };

    // getName()
    assert_loose_eq(&s("baz"), &get(&entity, "name"));
}
