//! Port of `InvalidMappingDefinitionTest`, on `SQLite`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/InvalidMappingDefinitionTest.php

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

use php2rust_doctrine_orm::mapping::mapping_exception;
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

#[tokio::test]
async fn testManyToManyRelationWithJoinTableOnTheWrongSide() {
    let em = orm::entity_manager(orm::connection(), None);

    // createSchemaForModels(): the metadata is loaded first.
    let error = em
        .get_class_metadata("Doctrine\\Tests\\ORM\\Functional\\OwningSideEntity")
        .and_then(|_| em.get_class_metadata("Doctrine\\Tests\\ORM\\Functional\\OwnedSideEntity"))
        .expect_err("expected a MappingException");

    assert!(
        thrown_is_a(&error, mapping_exception::CLASS),
        "{}",
        error.class
    );
    assert_eq!(
        orm::message(&error),
        "Mapping error on field 'owners' in Doctrine\\Tests\\ORM\\Functional\\OwnedSideEntity : 'joinTable' can only be set on many-to-many owning side."
    );
}
