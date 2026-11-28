//! Ticket/GH10661/GH10661Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; upstream's `OrmTestCase` with `getTestEntityManager()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10661/GH10661Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10661/InvalidChildEntity.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10661/InvalidEntity.php

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

use std::sync::Arc;

use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::schema_validator::SchemaValidator;

const INVALID_ENTITY: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10661\\InvalidEntity";
const INVALID_CHILD_ENTITY: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10661\\InvalidChildEntity";

fn set_up() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

fn bootstrap_validator(em: &Arc<EntityManager>, validate_property_types: bool) -> SchemaValidator {
    SchemaValidator::new(em, validate_property_types)
}

#[tokio::test]
async fn testMetadataFieldTypeNotCoherentWithEntityPropertyType() {
    let em = set_up();
    let class = em.get_class_metadata(INVALID_ENTITY).unwrap();
    let ce = bootstrap_validator(&em, true)
        .validate_class(&class)
        .unwrap();

    assert_eq!(
        vec![
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10661\\InvalidEntity#property1' has the property type 'float' that differs from the metadata field type 'string' returned by the 'decimal' DBAL type."
        ],
        ce
    );
}

#[tokio::test]
async fn testPropertyTypeErrorsCanBeSilenced() {
    let em = set_up();
    let class = em.get_class_metadata(INVALID_ENTITY).unwrap();
    let ce = bootstrap_validator(&em, false)
        .validate_class(&class)
        .unwrap();

    assert_eq!(ce, [] as [std::string::String; 0]);
}

#[tokio::test]
async fn testMetadataFieldTypeNotCoherentWithEntityPropertyTypeWithInheritance() {
    let em = set_up();
    let class = em.get_class_metadata(INVALID_CHILD_ENTITY).unwrap();
    let ce = bootstrap_validator(&em, true)
        .validate_class(&class)
        .unwrap();

    assert_eq!(
        vec![
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10661\\InvalidChildEntity#property1' has the property type 'float' that differs from the metadata field type 'string' returned by the 'decimal' DBAL type.",
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10661\\InvalidChildEntity#property2' has the property type 'int' that differs from the metadata field type 'string' returned by the 'string' DBAL type.",
            "The field 'Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10661\\InvalidChildEntity#anotherProperty' has the property type 'string' that differs from the metadata field type 'bool' returned by the 'boolean' DBAL type.",
        ],
        ce
    );
}
