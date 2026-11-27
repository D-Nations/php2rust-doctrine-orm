//! Port of `GH7684Test`, on `SQLite`: associations/columns with an inline
//! `_id` get named properly.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7684Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/database_driver_test_case.rs"]
mod database_driver_test_case;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::PhpArray;

use database_driver_test_case::{column, convert_to_class_metadata, foreign_key, metadata, table};

#[tokio::test]
async fn testIssue() {
    models::register();
    let em = orm::set_up(&[]).await;
    let table1 = table(
        "GH7684_identity_test_table",
        vec![column("id", "integer", &PhpArray::new())],
        &["id"],
        vec![],
    );

    let table2 = table(
        "GH7684_identity_test_assoc_table",
        vec![
            column("id", "integer", &PhpArray::new()),
            column("gh7684_identity_test_id", "integer", &PhpArray::new()),
        ],
        &["id"],
        vec![foreign_key(
            &["gh7684_identity_test_id"],
            "GH7684_identity_test_table",
            &["id"],
        )],
    );

    let metadatas = convert_to_class_metadata(&em, vec![table1, table2], vec![]).await;
    let metadata = metadata(&metadatas, "Gh7684IdentityTestAssocTable");

    assert!(
        metadata
            .association_mappings
            .get("gh7684IdentityTest")
            .is_some()
    );
}
