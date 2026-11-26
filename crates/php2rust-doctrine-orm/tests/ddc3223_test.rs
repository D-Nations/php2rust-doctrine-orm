//! Ticket/DDC3223Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `clone $participant->profileStatus` is the initialized proxy copied
//! into a new object of its class (PHP 8.4 initializes a lazy object
//! before cloning it).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3223Test.php

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
async fn testIssueGetId() {
    let em = ticket::set_up(&["Journalist", "Participant", "Status", "ProfileStatus"]).await;

    let profile_status = new("ProfileStatus");

    let participant = new("Journalist");
    set_obj(&participant, "profileStatus", &profile_status);

    em.persist(&profile_status).unwrap();
    em.persist(&participant).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let participant = find(&em, "Participant", get(&participant, "id"))
        .await
        .unwrap();

    let proxy = obj(&get(&participant, "profileStatus"));
    em.initialize_object(&proxy).await.unwrap();
    let profile_status = orm::new_object(proxy.class());
    set(&profile_status, "id", get(&proxy, "id"));

    assert!(
        matches!(get(&profile_status, "id"), Value::Int(1)),
        "The identifier on the cloned instance is an integer"
    );
}
