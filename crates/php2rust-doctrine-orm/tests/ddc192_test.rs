//! Ticket/DDC192Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC192Test.php

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

use php2rust_doctrine_dbal::schema::AbstractSchemaManager as _;
use ticket::*;

#[tokio::test]
async fn testSchemaCreation() {
    let classes = ["DDC192User", "DDC192Phonenumber"];

    let em = ticket::set_up(&classes).await;

    let tables = em
        .get_connection()
        .create_schema_manager()
        .await
        .unwrap()
        .list_table_names()
        .await
        .unwrap();

    for short in classes {
        let table = em
            .get_class_metadata(&class(short))
            .unwrap()
            .read()
            .get_table_name()
            .unwrap();
        assert!(tables.contains(&table), "{table} in {tables:?}");
    }
}
