//! CompanySchemaTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The schema of the `company` model set, introspected from `SQLite`.
//! `#[Depends('testGeneratedSchema')]` is a call of the test's body.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/SchemaTool/CompanySchemaTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php2rust_doctrine_dbal::schema::{AbstractSchemaManager, Schema};

use company::COMPANY_MODELS;

async fn generated_schema() -> Schema {
    let em = orm::set_up(&COMPANY_MODELS).await;
    let conn = em.get_connection();
    let schema = conn
        .create_schema_manager()
        .await
        .unwrap()
        .introspect_schema()
        .await
        .unwrap();

    assert!(schema.has_table("company_contracts"));

    schema
}

#[tokio::test]
async fn testGeneratedSchema() {
    generated_schema().await;
}

#[tokio::test]
async fn testSingleTableInheritance() {
    let schema = generated_schema().await;
    let table = schema.get_table("company_contracts").unwrap();

    // Check nullability constraints
    assert!(table.get_column("id").unwrap().get_notnull());
    assert!(table.get_column("completed").unwrap().get_notnull());
    assert!(!table.get_column("salesPerson_id").unwrap().get_notnull());
    assert!(table.get_column("discr").unwrap().get_notnull());
    assert!(!table.get_column("fixPrice").unwrap().get_notnull());
    assert!(!table.get_column("hoursWorked").unwrap().get_notnull());
    assert!(!table.get_column("pricePerHour").unwrap().get_notnull());
    assert!(!table.get_column("maxPrice").unwrap().get_notnull());
}
