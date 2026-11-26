//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/ScalarColumnHydratorTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/hydration.rs"]
mod hydration;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_SCALAR_COLUMN;
use php2rust_doctrine_orm::internal::hydration::MULTIPLE_SELECTORS_FOUND;
use php2rust_doctrine_orm::query::ResultSetMapping;

use hydration::{entity_manager, hydrate_all, rows};
use orm::{CMS_USER, at, list, message, s};

/// `Select u.id from CmsUser u`
#[tokio::test]
async fn testEmptyResultTest() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);

    let result = hydrate_all(&em, HYDRATE_SCALAR_COLUMN, Vec::new(), &rsm)
        .await
        .unwrap();
    assert!(list(&result).is_empty());
}

/// `Select u.id from CmsUser u`
#[tokio::test]
async fn testSingleColumnEntityQueryWithoutScalarMap() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);

    let result_set = rows(&[&[("u__id", s("1"))], &[("u__id", s("2"))]]);
    let result = hydrate_all(&em, HYDRATE_SCALAR_COLUMN, result_set, &rsm)
        .await
        .unwrap();
    assert_eq!(list(&result).len(), 2);
    orm::assert_loose_eq(&Value::Int(1), &at(&result, 0));
    orm::assert_loose_eq(&Value::Int(2), &at(&result, 1));
}

/// `Select u.id from CmsUser u`
#[tokio::test]
async fn testSingleColumnEntityQueryWithScalarMap() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);
    rsm.add_scalar_result("sclr0", "id", "string");
    rsm.add_index_by_scalar("sclr0");

    let result_set = rows(&[&[("u__id", s("1"))], &[("u__id", s("2"))]]);
    let result = hydrate_all(&em, HYDRATE_SCALAR_COLUMN, result_set, &rsm)
        .await
        .unwrap();
    assert_eq!(list(&result).len(), 2);
    orm::assert_loose_eq(&Value::Int(1), &at(&result, 0));
    orm::assert_loose_eq(&Value::Int(2), &at(&result, 1));
}

/// `Select u.id, u.name from CmsUser u`
#[tokio::test]
async fn testMultipleColumnEntityQueryThrowsException() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);
    rsm.add_field_result("u", "u__name", "name", None);

    let result_set = rows(&[
        &[("u__id", s("1")), ("u__name", s("Gregoire"))],
        &[("u__id", s("2")), ("u__name", s("Bhushan"))],
    ]);
    let e = hydrate_all(&em, HYDRATE_SCALAR_COLUMN, result_set, &rsm)
        .await
        .unwrap_err();
    assert_eq!(e.class, MULTIPLE_SELECTORS_FOUND);
    assert_eq!(
        message(&e),
        "Multiple selectors found: id, name. Please select only one."
    );
}
