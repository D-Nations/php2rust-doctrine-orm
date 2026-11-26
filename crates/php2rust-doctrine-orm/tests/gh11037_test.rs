//! Ticket/GH11037/GH11037Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; upstream's `OrmTestCase` with `getTestEntityManager()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11037/GH11037Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11037/EntityStatus.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11037/IntEntityStatus.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11037/InvalidEntityWithTypedEnum.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11037/StringEntityStatus.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11037/ValidEntityWithTypedEnum.php

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

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\";

#[tokio::test]
async fn testMetadataFieldTypeCoherentWithEntityPropertyType() {
    let em = orm::entity_manager(orm::connection(), None);
    let validator = SchemaValidator::new(&em, true);
    let class = em
        .get_class_metadata(&format!("{NS}ValidEntityWithTypedEnum"))
        .unwrap();
    let ce = validator.validate_class(&class).unwrap();

    assert!(ce.is_empty(), "{ce:?}");
}

#[tokio::test]
async fn testMetadataFieldTypeNotCoherentWithEntityPropertyType() {
    let em = orm::entity_manager(orm::connection(), None);
    let validator = SchemaValidator::new(&em, true);
    let class = em
        .get_class_metadata(&format!("{NS}InvalidEntityWithTypedEnum"))
        .unwrap();
    let ce = validator.validate_class(&class).unwrap();

    assert_eq!(
        vec![
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\InvalidEntityWithTypedEnum#status1' has the property type 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\StringEntityStatus' with a backing type of 'string' that differs from the metadata field type 'int'.",
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\InvalidEntityWithTypedEnum#status2' has the property type 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\IntEntityStatus' that differs from the metadata enumType 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\StringEntityStatus'.",
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\InvalidEntityWithTypedEnum#status3' has the metadata enumType 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH11037\\StringEntityStatus' with a backing type of 'string' that differs from the metadata field type 'int'.",
        ],
        ce
    );
}
