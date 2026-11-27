//! Ticket/GH8499Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `#[Depends]` runs the test it depends on first and
//! hands over its entity, with the entity manager cleared).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8499Test.php

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

use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_dbal::{LockMode, ParameterTypes};
use php2rust_doctrine_orm::collection_object::downcast;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

fn date_time(value: &Value) -> Arc<DateTime> {
    match value {
        Value::Object(object) => downcast::<DateTime>(object).expect("a DateTime"),
        other => panic!("not a DateTime: {other:?}"),
    }
}

async fn create_schema(em: &Arc<EntityManager>) {
    orm::create_schema(em, &[&class("GH8499VersionableEntity")]).await;
}

async fn optimistic_timestamp_sets_default_value(em: &Arc<EntityManager>) -> Object {
    create_schema(em).await;
    let entity = new("GH8499VersionableEntity");
    set(&entity, "name", s("Test Entity"));
    set(
        &entity,
        "description",
        s("Entity to test optimistic lock fix with DateTimeInterface objects"),
    );
    assert!(get(&entity, "revision").is_null(), "Pre-Condition");

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    date_time(&get(&entity, "revision"));

    entity
}

#[tokio::test]
async fn testOptimisticTimestampSetsDefaultValue() {
    let em = orm::entity_manager(orm::connection(), None);
    optimistic_timestamp_sets_default_value(&em).await;
}

#[tokio::test]
async fn testOptimisticLockWithDateTimeForVersion() {
    let em = orm::entity_manager(orm::connection(), None);
    let entity = optimistic_timestamp_sets_default_value(&em).await;
    em.clear().unwrap();

    let q = em.create_query(
        "SELECT t FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH8499VersionableEntity t WHERE t.id = :id",
    );
    q.set_parameter("id", get(&entity, "id"), None);
    let test = obj(&q.get_single_result(None).await.unwrap());

    let format = em.get_platform().get_date_time_format_string();
    let modified = DateTimeImmutable::from_timestamp(
        date_time(&get(&test, "revision")).get().timestamp() - 3600,
        0,
        TimeZone::Utc,
    );
    let modified_date = DateTime::object(modified);

    em.get_connection()
        .execute_statement(
            "UPDATE GH8499VersionableEntity SET revision = ? WHERE id = ?",
            &PhpArray::list(vec![s(&modified.format(&format)), get(&test, "id")]),
            &ParameterTypes::new(),
        )
        .await
        .unwrap();

    em.refresh(&test).await.unwrap();
    em.lock(&test, LockMode::Optimistic, Some(&modified_date))
        .await
        .unwrap();

    set(&test, "name", s("Test Entity Locked"));
    em.persist(&test).unwrap();
    em.flush().await.unwrap();

    assert_loose_eq(&s("Test Entity Locked"), &get(&test, "name"));
    assert!(
        date_time(&get(&test, "revision")).get().timestamp() > modified.timestamp(),
        "Current version timestamp is not greater than previous one."
    );
}

#[tokio::test]
async fn testOptimisticLockWithDateTimeForVersionThrowsException() {
    let em = orm::entity_manager(orm::connection(), None);
    create_schema(&em).await;
    let entity = new("GH8499VersionableEntity");
    set(&entity, "name", s("Test Entity"));
    set(
        &entity,
        "description",
        s("Entity to test optimistic lock fix with DateTimeInterface objects"),
    );
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    let version = DateTime::object(DateTimeImmutable::from_timestamp(
        1_594_836_240, // 2020-07-15 18:04:00 UTC
        0,
        TimeZone::Utc,
    ));
    let e = em
        .lock(&entity, LockMode::Optimistic, Some(&version))
        .await
        .unwrap_err();
    assert_eq!("Doctrine\\ORM\\OptimisticLockException", e.class, "{e:?}");
}
