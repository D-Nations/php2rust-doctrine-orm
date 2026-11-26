//! Ticket/DDC698Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC698Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testTicket() {
    let em = ticket::set_up(&["DDC698Role", "DDC698Privilege"]).await;

    let mut qb = em.create_query_builder();
    qb.select(vec![s("p"), s("r")])
        .unwrap()
        .from(&class("DDC698Privilege"), "p", None)
        .unwrap()
        .left_join("p.roles", "r", None, None, None)
        .unwrap();

    let SqlStatements::One(sql) = qb.get_query().unwrap().get_sql().unwrap() else {
        panic!("one statement");
    };

    assert_eq!(
        "SELECT p0_.privilegeID AS privilegeID_0, p0_.name AS name_1, r1_.roleID AS roleID_2, r1_.name AS name_3, r1_.shortName AS shortName_4 FROM Privileges p0_ LEFT JOIN RolePrivileges r2_ ON p0_.privilegeID = r2_.privilegeID LEFT JOIN Roles r1_ ON r1_.roleID = r2_.roleID".to_lowercase(),
        sql.to_lowercase(),
    );
}
