//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SchemaTool/DBAL483Test.php
//!
//! On `SQLite` (`tests/common/orm.rs`); `DBAL483Default` is the recorded
//! model of the test file.

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

/// DBAL-483
#[tokio::test]
async fn testDefaultValueIsComparedCorrectly() {
    let class = DBAL483_DEFAULT;

    let em = orm::set_up(&[class]).await;

    let metadata = vec![em.get_class_metadata(class).unwrap()];
    let update_sql = SchemaTool::new(&em)
        .get_update_schema_sql(&metadata)
        .await
        .unwrap();

    let update_sql: Vec<String> = update_sql
        .into_iter()
        .filter(|sql| sql.contains("DBAL483"))
        .collect();

    assert_eq!(0, update_sql.len(), "{update_sql:#?}");
}
