//! Ticket/DDC1225Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1225Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::query::exec::SqlStatements;

use orm::*;

const ENTITY1: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1225TestEntity1";
const ENTITY2: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1225TestEntity2";

#[tokio::test]
async fn testIssue() {
    let em = orm::set_up(&[ENTITY1, ENTITY2]).await;

    let mut qb = em.create_query_builder();
    qb.from(ENTITY1, "te1", None)
        .unwrap()
        .select(vec![s("te1")])
        .unwrap()
        .r#where(vec![s("te1.testEntity2 = ?1")])
        .unwrap()
        .set_parameter(1, Value::Int(0), None);

    let SqlStatements::One(sql) = qb.get_query().unwrap().get_sql().unwrap() else {
        panic!("one statement");
    };
    assert_eq!(
        "SELECT t0_.test_entity2_id AS test_entity2_id_0 FROM te1 t0_ WHERE t0_.test_entity2_id = ?"
            .to_lowercase(),
        sql.to_lowercase()
    );
}
