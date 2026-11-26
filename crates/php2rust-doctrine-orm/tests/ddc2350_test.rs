//! Ticket/DDC2350Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2350Test.php

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/ticket.rs"]
mod ticket;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testEagerCollectionsAreOnlyRetrievedOnce() {
    let classes = [class("DDC2350User"), class("DDC2350Bug")];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let user = new("DDC2350User");
    let bug1 = new("DDC2350Bug");
    set_obj(&bug1, "user", &user);
    let bug2 = new("DDC2350Bug");
    set_obj(&bug2, "user", &user);

    em.persist(&user).unwrap();
    em.persist(&bug1).unwrap();
    em.persist(&bug2).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    log.reset_and_enable();
    let user = find(&em, "DDC2350User", get(&user, "id")).await.unwrap();
    assert_eq!(2, log.queries().len());

    assert_eq!(2, count_of(&user, "reportedBugs").await);

    assert_eq!(2, log.queries().len());
}
