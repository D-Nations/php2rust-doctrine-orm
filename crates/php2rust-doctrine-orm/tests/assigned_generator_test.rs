//! Port of tests/Tests/ORM/Id/AssignedGeneratorTest.php, with the
//! recorded models of the test file. `getTestEntityManager()` is an
//! entity manager on an in-memory `SQLite` database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Id/AssignedGeneratorTest.php

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

use php_value::{Object, Value, php_array};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::exception::ENTITY_MISSING_ASSIGNED_ID;
use php2rust_doctrine_orm::id::AssignedGenerator;

const NS: &str = "Doctrine\\Tests\\ORM\\Id\\";

struct Test {
    entity_manager: Arc<EntityManager>,
    assigned_gen: AssignedGenerator,
}

fn set_up() -> Test {
    Test {
        entity_manager: orm::entity_manager(orm::connection(), None),
        assigned_gen: AssignedGenerator::new(),
    }
}

fn new(short: &str) -> Object {
    orm::new_object(&format!("{NS}{short}"))
}

/// `entitiesWithoutId()`.
fn entities_without_id() -> Vec<(&'static str, Object)> {
    vec![
        ("single", new("AssignedSingleIdEntity")),
        ("composite", new("AssignedCompositeIdEntity")),
    ]
}

#[test]
fn testThrowsExceptionIfIdNotAssigned() {
    for (name, entity) in entities_without_id() {
        let t = set_up();

        // `EntityMissingAssignedId` is an `ORMException`.
        let e = t
            .assigned_gen
            .generate_id(&t.entity_manager, &entity)
            .unwrap_err();
        assert_eq!(e.class, ENTITY_MISSING_ASSIGNED_ID, "{name}");
    }
}

#[test]
fn testCorrectIdGeneration() {
    let t = set_up();
    let entity = new("AssignedSingleIdEntity");
    orm::set(&entity, "myId", Value::Int(1));
    let id = t
        .assigned_gen
        .generate_id(&t.entity_manager, &entity)
        .unwrap();
    assert_eq!(id, php_array!["myId" => 1]);

    let entity = new("AssignedCompositeIdEntity");
    orm::set(&entity, "myId2", Value::Int(2));
    orm::set(&entity, "myId1", Value::Int(4));
    let id = t
        .assigned_gen
        .generate_id(&t.entity_manager, &entity)
        .unwrap();
    assert_eq!(id, php_array!["myId1" => 4, "myId2" => 2]);
}
