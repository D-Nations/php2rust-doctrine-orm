//! Ticket/Ticket4646InstanceOfWithMultipleParametersTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/Ticket4646InstanceOfWithMultipleParametersTest.php

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
    let em = ticket::set_up(&[
        "PersonTicket4646Multiple",
        "EmployeeTicket4646Multiple",
        "ManagerTicket4646Multiple",
        "InternTicket4646Multiple",
    ])
    .await;
    em.persist(&new("PersonTicket4646Multiple")).unwrap();
    em.persist(&new("EmployeeTicket4646Multiple")).unwrap();
    em.persist(&new("ManagerTicket4646Multiple")).unwrap();
    em.persist(&new("InternTicket4646Multiple")).unwrap();
    em.flush().await.unwrap();

    let dql = "SELECT p FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\PersonTicket4646Multiple p
                WHERE p INSTANCE OF (Doctrine\\Tests\\ORM\\Functional\\Ticket\\EmployeeTicket4646Multiple, Doctrine\\Tests\\ORM\\Functional\\Ticket\\InternTicket4646Multiple)";
    let query = em.create_query(dql);
    let result = list(&query.get_object_result().await.unwrap());

    assert_eq!(2, result.len());
    for entity in &result {
        assert_instance_of("PersonTicket4646Multiple", entity);
    }
}
