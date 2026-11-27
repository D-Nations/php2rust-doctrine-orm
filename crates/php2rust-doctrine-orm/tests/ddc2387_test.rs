//! Port of `DDC2387Test`, on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2387Test.php

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
use php2rust_doctrine_orm::mapping::class_metadata as cm;

use database_driver_test_case::{column, convert_to_class_metadata, foreign_key, metadata, table};

#[tokio::test]
async fn testCompositeAssociationKeyDetection() {
    models::register();
    let em = orm::set_up(&[]).await;
    let product = table(
        "ddc2387_product",
        vec![column("id", "integer", &PhpArray::new())],
        &["id"],
        vec![],
    );

    let attributes = table(
        "ddc2387_attributes",
        vec![
            column("product_id", "integer", &PhpArray::new()),
            column("attribute_name", "string", &PhpArray::new()),
        ],
        &["product_id", "attribute_name"],
        vec![foreign_key(
            &["product_id"],
            "ddc2387_product",
            &["product_id"],
        )],
    );

    let metadata_list = convert_to_class_metadata(&em, vec![product, attributes], vec![]).await;

    assert_eq!(
        cm::GENERATOR_TYPE_NONE,
        metadata(&metadata_list, "Ddc2387Attributes").generator_type
    );
    assert_eq!(
        cm::GENERATOR_TYPE_AUTO,
        metadata(&metadata_list, "Ddc2387Product").generator_type
    );
}
