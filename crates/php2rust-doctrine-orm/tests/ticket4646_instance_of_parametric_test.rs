//! Ticket/Ticket4646InstanceOfParametricTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/Ticket4646InstanceOfParametricTest.php

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
async fn testInstanceOf() {
    let em = ticket::set_up(&["PersonTicket4646Parametric", "EmployeeTicket4646Parametric"]).await;
    em.persist(&new("PersonTicket4646Parametric")).unwrap();
    em.persist(&new("EmployeeTicket4646Parametric")).unwrap();
    em.flush().await.unwrap();

    let dql = "SELECT p FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\PersonTicket4646Parametric p
                WHERE p INSTANCE OF :parameter";
    let query = em.create_query(dql);
    query.set_parameter(
        "parameter",
        Value::Object(em.get_class_metadata(&class("PersonTicket4646Parametric")).unwrap()),
        None,
    );
    let result = list(&query.get_object_result().await.unwrap());

    assert_eq!(2, result.len());
    for entity in &result {
        assert_instance_of("PersonTicket4646Parametric", entity);
    }
}
