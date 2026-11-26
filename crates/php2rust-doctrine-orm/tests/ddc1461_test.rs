//! Ticket/DDC1461Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1461Test.php

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
use php2rust_doctrine_orm::unit_of_work::{STATE_MANAGED, STATE_NEW};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testChangeDetectionDeferredExplicit() {
    let em = ticket::set_up(&["DDC1461TwitterAccount", "DDC1461User"]).await;

    let user = new("DDC1461User");
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let uow = em.get_unit_of_work();
    assert_eq!(
        STATE_MANAGED,
        uow.get_entity_state_or(&user, STATE_NEW),
        "Entity should be managed."
    );
    assert_eq!(
        STATE_MANAGED,
        uow.get_entity_state(&user).await.unwrap(),
        "Entity should be managed."
    );

    let acc = new("DDC1461TwitterAccount");
    set_obj(&user, "twitterAccount", &acc);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    let user = find(&em, "DDC1461User", get(&user, "id")).await.unwrap();
    assert_ne!(Value::Null, get(&user, "twitterAccount"));
}
