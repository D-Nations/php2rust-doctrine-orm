//! Ticket/GH11072/GH11072Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; upstream's `getTestEntityManager()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11072/GH11072Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11072/GH11072EntityAdvanced.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11072/GH11072EntityBasic.php

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

use php2rust_doctrine_orm::tools::schema_validator::SchemaValidator;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11072\\";

#[tokio::test]
async fn testAcceptsSubsetOfBuiltinTypesWithoutErrors() {
    let em = orm::entity_manager(orm::connection(), None);
    let validator = SchemaValidator::new(&em, true);
    let class = em
        .get_class_metadata(&format!("{NS}GH11072EntityBasic"))
        .unwrap();
    let ce = validator.validate_class(&class).unwrap();

    assert!(ce.is_empty(), "{ce:?}");
}

#[tokio::test]
async fn testAcceptsAdvancedSubsetOfBuiltinTypesWithoutErrors() {
    let em = orm::entity_manager(orm::connection(), None);
    let validator = SchemaValidator::new(&em, true);
    let class = em
        .get_class_metadata(&format!("{NS}GH11072EntityAdvanced"))
        .unwrap();
    let ce = validator.validate_class(&class).unwrap();

    assert!(ce.is_empty(), "{ce:?}");
}
