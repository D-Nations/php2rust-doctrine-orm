//! Ticket/GH12438Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the Models/GH12438 models).
//!
//! The `DateTimeImmutable` values are in UTC.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12438Test.php
//! Upstream: doctrine/orm tests/Tests/Models/GH12438/VersionableEntity.php
//! Upstream: doctrine/orm tests/Tests/Models/GH12438/VersionEntity.php

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

use php_value::datetime::{DateTimeImmutable, Fields, TimeZone};
use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_orm::query::expr::Join;

use orm::*;

const VERSIONABLE_ENTITY: &str = "Doctrine\\Tests\\Models\\GH12438\\VersionableEntity";
const VERSION_ENTITY: &str = "Doctrine\\Tests\\Models\\GH12438\\VersionEntity";

/// `new DateTimeImmutable('Y-m-d')`.
fn date(year: i64, month: i64, day: i64) -> Value {
    DateTimeImmutable::from_fields(
        Fields {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            second: 0,
            microsecond: 0,
        },
        TimeZone::Utc,
    )
    .object()
}

/// `new VersionableEntity($id)`.
fn versionable(id: i64) -> Object {
    let versionable = new_object(VERSIONABLE_ENTITY);
    set(&versionable, "id", Value::Int(id));
    versionable
}

/// `new VersionEntity($id)`, `setVersionDate()` and `setVersionable()`.
fn version(id: i64, version_date: Value, versionable: &Object) -> Object {
    let version = new_object(VERSION_ENTITY);
    set(&version, "id", Value::Int(id));
    set(&version, "versionDate", version_date);
    set_obj(&version, "versionable", versionable);
    version
}

/// `setUp()` with `generateFixture()`.
async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&[VERSIONABLE_ENTITY, VERSION_ENTITY]).await;

    let versionable1 = versionable(1);
    let version11 = version(11, date(2024, 2, 1), &versionable1);
    let version12 = version(12, date(2024, 1, 1), &versionable1);
    let version13 = version(13, date(2025, 3, 2), &versionable1);

    let versionable2 = versionable(2);
    let version21 = version(21, date(2025, 4, 1), &versionable2);
    let version22 = version(22, date(2025, 3, 1), &versionable2);
    let version23 = version(23, date(2025, 6, 1), &versionable2);
    let version24 = version(24, date(2025, 5, 1), &versionable2);

    for entity in [
        &versionable1,
        &version11,
        &version12,
        &version13,
        &versionable2,
        &version21,
        &version22,
        &version23,
        &version24,
    ] {
        em.persist(entity).unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    em
}

/// `assertVersionable()`.
async fn assert_versionable(versionable: &Value, expected_ids: &[i64]) {
    // getVersions(): the hydrated collection
    let versions = get(&obj(versionable), "versions");

    assert_eq!(expected_ids.len(), coll::count(&versions).await.unwrap());

    for (index, expected_id) in expected_ids.iter().enumerate() {
        let version = coll::get(&versions, i64::try_from(index).unwrap())
            .await
            .unwrap();

        assert!(
            matches!(&version, Value::Object(o) if o.instance_of(VERSION_ENTITY)),
            "{version:?}"
        );
        assert_loose_eq(&Value::Int(*expected_id), &get(&obj(&version), "id"));
    }
}

#[tokio::test]
async fn testFetchAll() {
    let em = set_up().await;

    let mut qb = em.create_query_builder();
    qb.select(vec![s("versionable"), s("version")])
        .unwrap()
        .from(VERSIONABLE_ENTITY, "versionable", None)
        .unwrap()
        .inner_join("versionable.versions", "version", None, None, None)
        .unwrap()
        .order_by("versionable.id", Some(SortDirection::Descending.into()))
        .unwrap();
    let versionables = list(&qb.get_query().unwrap().get_object_result().await.unwrap());

    assert_eq!(2, versionables.len());

    assert_versionable(&versionables[0], &[23, 24, 21, 22]).await;
    assert_versionable(&versionables[1], &[13, 11, 12]).await;
}

/// The join condition of both `testFetchWithJoinCondition*` tests.
fn condition() -> String {
    format!(
        "version.id IN (SELECT v.id FROM {VERSION_ENTITY} AS v WHERE v.versionDate > :date ORDER BY v.id DESC)"
    )
}

#[tokio::test]
async fn testFetchWithJoinCondition() {
    let em = set_up().await;

    let mut qb = em.create_query_builder();
    qb.select(vec![s("versionable"), s("version")])
        .unwrap()
        .from(VERSIONABLE_ENTITY, "versionable", None)
        .unwrap()
        .left_join(
            "versionable.versions",
            "version",
            Some(Join::WITH),
            Some(s(&condition())),
            None,
        )
        .unwrap()
        .set_parameter("date", date(2025, 3, 1), None)
        .order_by("versionable.id", Some(SortDirection::Descending.into()))
        .unwrap();
    let versionables = list(&qb.get_query().unwrap().get_object_result().await.unwrap());

    assert_versionable(&versionables[0], &[23, 24, 21]).await;
    assert_versionable(&versionables[1], &[13]).await;
}

#[tokio::test]
async fn testFetchWithJoinConditionWithoutAssociation() {
    let em = set_up().await;

    let mut qb = em.create_query_builder();
    qb.distinct(true)
        .select(vec![s("version.id")])
        .unwrap()
        .from(VERSIONABLE_ENTITY, "versionable", None)
        .unwrap()
        .left_join(
            VERSION_ENTITY,
            "version",
            Some(Join::ON),
            Some(s(&condition())),
            None,
        )
        .unwrap()
        .set_parameter("date", date(2025, 3, 1), None)
        .order_by("version.id", Some(SortDirection::Ascending.into()))
        .unwrap();
    let versionables = qb
        .get_query()
        .unwrap()
        .get_single_column_result()
        .await
        .unwrap();

    assert!(matches!(versionables, Value::Array(_)));
    let versionables = list(&versionables);
    assert_eq!(4, versionables.len());
    for (expected, actual) in [13, 21, 23, 24].iter().zip(&versionables) {
        assert_loose_eq(&Value::Int(*expected), actual);
    }
}
