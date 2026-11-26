//! Ticket/DDC2138Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `class_exists(ForeignKeyConstraintEditor::class)` is true for the
//! DBAL port, so the test takes the DBAL 4 branch.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2138Test.php

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

use php2rust_doctrine_dbal::schema::Table as DbalTable;
use php2rust_doctrine_orm::tools::schema_tool::SchemaTool;

use ticket::*;

fn column_is_indexed(table: &DbalTable, column: &str) -> bool {
    table
        .get_indexes()
        .values()
        .any(|index| index.spans_columns(&[column.to_owned()]))
}

#[tokio::test]
async fn testForeignKeyOnSTIWithMultipleMapping() {
    let em = orm::entity_manager(orm::connection(), None);
    let classes: Vec<_> = [
        "DDC2138User",
        "DDC2138Structure",
        "DDC2138UserFollowedObject",
        "DDC2138UserFollowedStructure",
        "DDC2138UserFollowedUser",
    ]
    .iter()
    .map(|short| em.get_class_metadata(&class(short)).unwrap())
    .collect();
    let schema = SchemaTool::new(&em)
        .get_schema_from_metadata(&classes)
        .unwrap();

    assert!(
        schema.has_table("users_followed_objects"),
        "Table users_followed_objects should exist."
    );

    let table = schema.get_table("users_followed_objects").unwrap();

    assert!(column_is_indexed(table, "object_id"));
    assert!(column_is_indexed(table, "user_id"));
    let foreign_keys = table.get_foreign_keys();
    assert_eq!(
        1,
        foreign_keys.len(),
        "user_id column has to have FK, but not object_id"
    );

    let fk = foreign_keys.values().next().unwrap();

    assert_eq!("users", fk.get_referenced_table_name().unwrap().to_string());
    let local_columns: Vec<String> = fk
        .get_referencing_column_names()
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();

    assert!(local_columns.contains(&"user_id".to_owned()));
    assert_eq!(1, local_columns.len());
}
