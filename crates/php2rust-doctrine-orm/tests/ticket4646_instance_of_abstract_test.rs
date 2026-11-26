//! Ticket/Ticket4646InstanceOfAbstractTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/Ticket4646InstanceOfAbstractTest.php

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
async fn testInstanceOf() {
    let em = ticket::set_up(&["PersonTicket4646Abstract", "EmployeeTicket4646Abstract"]).await;
    em.persist(&new("EmployeeTicket4646Abstract")).unwrap();
    em.flush().await.unwrap();

    let dql = "SELECT p FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\PersonTicket4646Abstract p
                WHERE p INSTANCE OF Doctrine\\Tests\\ORM\\Functional\\Ticket\\PersonTicket4646Abstract";
    let query = em.create_query(dql);
    let result = list(&query.get_object_result().await.unwrap());

    assert_eq!(1, result.len());
    for entity in &result {
        assert_instance_of("PersonTicket4646Abstract", entity);
    }
}
