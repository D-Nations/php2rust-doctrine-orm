//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/ScalarHydratorTest.php

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

use php2rust_doctrine_orm::abstract_query::HYDRATE_SCALAR;
use php2rust_doctrine_orm::query::ResultSetMapping;

use hydration::{entity_manager, hydrate_all, rows};
use orm::{CMS_USER, at, list, s};

/// Select u.id, u.name from CmsUser u
#[tokio::test]
async fn testNewHydrationSimpleEntityQuery() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);
    rsm.add_field_result("u", "u__name", "name", None);

    // Faked result set
    let result_set = rows(&[
        &[("u__id", s("1")), ("u__name", s("romanb"))],
        &[("u__id", s("2")), ("u__name", s("jwage"))],
    ]);

    let result = hydrate_all(&em, HYDRATE_SCALAR, result_set, &rsm).await.unwrap();

    assert_eq!(list(&result).len(), 2);
    assert_eq!(at(&at(&result, 0), "u_name"), s("romanb"));
    orm::assert_loose_eq(&1.into(), &at(&at(&result, 0), "u_id"));
    assert_eq!(at(&at(&result, 1), "u_name"), s("jwage"));
    orm::assert_loose_eq(&2.into(), &at(&at(&result, 1), "u_id"));
}

#[tokio::test]
async fn testHydrateScalarResults() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_scalar_result("foo1", "foo", "string");
    rsm.add_scalar_result("bar2", "bar", "string");
    rsm.add_scalar_result("baz3", "baz", "string");

    let result_set = rows(&[&[("foo1", s("A")), ("bar2", s("B")), ("baz3", s("C"))]]);

    let result = hydrate_all(&em, HYDRATE_SCALAR, result_set, &rsm).await.unwrap();
    assert_eq!(list(&result).len(), 1);
}

#[tokio::test]
async fn testSkipUnknownColumns() {
    let em = entity_manager();
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);
    rsm.add_field_result("u", "u__name", "name", None);
    rsm.add_scalar_result("foo1", "foo", "string");
    rsm.add_scalar_result("bar2", "bar", "string");
    rsm.add_scalar_result("baz3", "baz", "string");

    let result_set = rows(&[&[
        ("u__id", s("1")),
        ("u__name", s("romanb")),
        ("foo1", s("A")),
        ("bar2", s("B")),
        ("baz3", s("C")),
        ("foo", s("bar")), // Unknown!
    ]]);

    let result = hydrate_all(&em, HYDRATE_SCALAR, result_set, &rsm).await.unwrap();
    assert_eq!(list(&result).len(), 1);
}
