//! Ticket/DDC1250Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1250Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC1250ClientHistory"]).await;

    let c1 = new("DDC1250ClientHistory");
    let c2 = new("DDC1250ClientHistory");
    set_obj(&c1, "declinedClientsHistory", &c2);
    set_obj(&c1, "declinedBy", &c2);
    set_obj(&c2, "declinedBy", &c1);
    set_obj(&c2, "declinedClientsHistory", &c1);

    em.persist(&c1).unwrap();
    em.persist(&c2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!(
        "SELECT h FROM {NS}DDC1250ClientHistory h WHERE h.id = ?1"
    ));
    query.set_parameter(1, get(&c2, "id"), None);
    let history = query.get_single_result(None).await.unwrap();

    assert_instance_of("DDC1250ClientHistory", &history);
}
