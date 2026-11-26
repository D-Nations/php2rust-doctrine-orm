//! Ticket/DDC513Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC513Test.php

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
    let em = ticket::set_up(&["DDC513OfferItem", "DDC513Item", "DDC513Price"]).await;

    let q = em.create_query(&format!(
        "select u from {NS}DDC513OfferItem u left join u.price p"
    ));
    let SqlStatements::One(sql) = q.get_sql().unwrap() else {
        panic!("one statement");
    };
    assert_eq!(
        "SELECT d0_.id AS id_0, d0_.discr AS discr_1, d0_.price AS price_2 FROM DDC513OfferItem d1_ INNER JOIN DDC513Item d0_ ON d1_.id = d0_.id LEFT JOIN DDC513Price d2_ ON d0_.price = d2_.id".to_lowercase(),
        sql.to_lowercase(),
    );
}
