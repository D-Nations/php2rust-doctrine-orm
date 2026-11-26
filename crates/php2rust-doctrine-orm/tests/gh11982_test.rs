//! Ticket/GH11982Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `testSchemaLegacyDbal` is `n/a`: upstream skips it on doctrine/dbal
//! 4.3 and later, which the DBAL port follows (`Index::getIndexedColumns()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11982Test.php

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

use php2rust_doctrine_dbal::schema::AbstractSchemaManager;

#[tokio::test]
async fn testSchema() {
    let em = ticket::set_up(&["GH11982ColumnIndex"]).await;

    let mut indexes = em
        .get_connection()
        .create_schema_manager()
        .await
        .unwrap()
        .introspect_table("GH11982ColumnIndex")
        .await
        .unwrap()
        .get_indexes()
        .clone();

    assert_eq!(3, indexes.len()); // primary + 2 custom indexes
    assert!(indexes.contains_key("class_idx"));

    indexes.remove("primary");
    indexes.remove("class_idx");
    let index = indexes.values().next().unwrap();
    let unnamed_index_columns = index.get_indexed_columns().unwrap();
    assert_eq!(1, unnamed_index_columns.len());
    assert_eq!(
        "indexTrue",
        unnamed_index_columns[0].get_column_name().to_string()
    );
}
