//! Ticket/DDC493Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC493Test.php

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

use php2rust_doctrine_orm::query::exec::SqlStatements;

use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC493Customer", "DDC493Distributor", "DDC493Contact"]).await;

    let q = em.create_query(&format!(
        "select u, c.data from {NS}DDC493Distributor u JOIN u.contact c"
    ));
    let SqlStatements::One(sql) = q.get_sql().unwrap() else {
        panic!("one statement");
    };
    assert_eq!(
        "SELECT d0_.id AS id_0, d1_.data AS data_1, d0_.discr AS discr_2, d0_.contact AS contact_3 FROM DDC493Distributor d2_ INNER JOIN DDC493Customer d0_ ON d2_.id = d0_.id INNER JOIN DDC493Contact d1_ ON d0_.contact = d1_.id".to_lowercase(),
        sql.to_lowercase(),
    );
}
