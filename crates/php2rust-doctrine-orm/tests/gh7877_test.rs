//! Port of `GH7877Test`, on `SQLite`. The constructors' `uniqid()` is a
//! counter-based id.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/GH7877Test.php

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
use std::sync::atomic::{AtomicU64, Ordering};

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{new_object, s, set};

const APPLICATION_GENERATED_ID: &str =
    "Doctrine\\Tests\\ORM\\Functional\\GH7877ApplicationGeneratedIdEntity";
const NULLABLE_ASSOCIATION: &str =
    "Doctrine\\Tests\\ORM\\Functional\\GH7877EntityWithNullableAssociation";

/// `uniqid()`: a unique id.
fn uniqid() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    format!("gh7877-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// `new $class()`: the constructor sets the id.
fn entity(class: &str) -> Object {
    let entity = new_object(class);
    set(&entity, "id", s(&uniqid()));
    entity
}

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[APPLICATION_GENERATED_ID, NULLABLE_ASSOCIATION]).await
}

#[tokio::test]
async fn testSelfReferenceWithApplicationGeneratedIdMayBeNotNullable() {
    let em = set_up().await;
    let entity = entity(APPLICATION_GENERATED_ID);
    set(&entity, "parent", Value::Object(entity.clone()));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
}

#[tokio::test]
async fn testCrossReferenceWithApplicationGeneratedIdMayBeNotNullable() {
    let em = set_up().await;
    let entity1 = entity(APPLICATION_GENERATED_ID);
    set(&entity1, "parent", Value::Object(entity1.clone()));
    let entity2 = entity(APPLICATION_GENERATED_ID);
    set(&entity2, "parent", Value::Object(entity1.clone()));

    // As long as we do not have entity-level commit order computation
    // (see https://github.com/doctrine/orm/pull/10547),
    // this only works when the UoW processes $entity1 before $entity2,
    // so that the foreign key constraint E2 -> E1 can be satisfied.

    em.persist(&entity1).unwrap();
    em.persist(&entity2).unwrap();
    em.flush().await.unwrap();
}

#[tokio::test]
async fn testNullableForeignKeysMakeInsertOrderLessRelevant() {
    let em = set_up().await;
    let entity1 = entity(NULLABLE_ASSOCIATION);
    set(&entity1, "parent", Value::Object(entity1.clone()));
    let entity2 = entity(NULLABLE_ASSOCIATION);
    set(&entity2, "parent", Value::Object(entity1.clone()));

    // In contrast to the previous test, this case demonstrates that with NULLable
    // associations, even without entity-level commit order computation
    // (see https://github.com/doctrine/orm/pull/10547), we can get away with an
    // insertion order of E2 before E1. That is because the UoW will schedule an extra
    // update that saves the day - the foreign key reference will established only after
    // all insertions have been performed.

    em.persist(&entity2).unwrap();
    em.persist(&entity1).unwrap();
    em.flush().await.unwrap();
}
