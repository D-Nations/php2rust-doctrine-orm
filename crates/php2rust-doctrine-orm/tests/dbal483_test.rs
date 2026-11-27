//! DBAL483Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own model).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SchemaTool/DBAL483Test.php

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

use php2rust_doctrine_orm::tools::SchemaTool;

const DBAL483_DEFAULT: &str = "Doctrine\\Tests\\ORM\\Functional\\SchemaTool\\DBAL483Default";

#[tokio::test]
async fn testDefaultValueIsComparedCorrectly() {
    let em = orm::set_up(&[DBAL483_DEFAULT]).await;

    let class = em.get_class_metadata(DBAL483_DEFAULT).unwrap();
    let update_sql: Vec<String> = SchemaTool::new(&em)
        .get_update_schema_sql(&[class])
        .await
        .unwrap()
        .into_iter()
        .filter(|sql| sql.contains("DBAL483"))
        .collect();

    assert_eq!(0, update_sql.len(), "{update_sql:?}");
}
