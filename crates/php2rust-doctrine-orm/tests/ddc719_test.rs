//! Ticket/DDC719Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC719Test.php

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

#[tokio::test]
async fn testIsEmptySqlGeneration() {
    let em = ticket::set_up(&["DDC719Group"]).await;

    let q = em.create_query(
        "SELECT g, c FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC719Group g LEFT JOIN g.children c  WHERE g.parents IS EMPTY",
    );

    let reference_sql = "SELECT g0_.id AS id_0, g0_.name AS name_1, g0_.description AS description_2, g1_.id as id_3, g1_.name AS name_4, g1_.description AS description_5 FROM groups g0_ LEFT JOIN groups_groups g2_ ON g0_.id = g2_.parent_id LEFT JOIN groups g1_ ON g1_.id = g2_.child_id WHERE (SELECT COUNT(*) FROM groups_groups g3_ WHERE g3_.child_id = g0_.id) = 0";

    let SqlStatements::One(sql) = q.get_sql().unwrap() else {
        panic!("one statement");
    };
    assert_eq!(reference_sql.to_lowercase(), sql.to_lowercase());
}
