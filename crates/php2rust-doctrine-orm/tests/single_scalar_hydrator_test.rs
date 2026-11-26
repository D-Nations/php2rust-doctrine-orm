//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/SingleScalarHydratorTest.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::{HYDRATE_SINGLE_SCALAR, NON_UNIQUE_RESULT_EXCEPTION};
use php2rust_doctrine_orm::query::ResultSetMapping;

use hydration::{entity_manager, hydrate_all, rows};
use orm::{CMS_USER, s};

fn valid_result_set_provider() -> Vec<(Vec<PhpArray>, Value)> {
    vec![
        // SELECT u.name FROM CmsUser u WHERE u.id = 1
        (rows(&[&[("u__name", s("romanb"))]]), s("romanb")),
        // SELECT u.id FROM CmsUser u WHERE u.id = 1
        (rows(&[&[("u__id", s("1"))]]), Value::Int(1)),
        // SELECT u.id, COUNT(u.postsCount + u.likesCount) AS HIDDEN score
        // FROM CmsUser u WHERE u.id = 1
        (
            rows(&[&[
                ("u__id", s("1")),
                // Ignored since not part of ResultSetMapping (cf. HIDDEN keyword)
                ("score", Value::Int(10)),
            ]]),
            Value::Int(1),
        ),
    ]
}

fn invalid_result_set_provider() -> Vec<Vec<PhpArray>> {
    vec![
        // Single row (OK), multiple columns (NOT OK)
        rows(&[&[("u__id", s("1")), ("u__name", s("romanb"))]]),
        // Multiple rows (NOT OK), single column (OK)
        rows(&[&[("u__id", s("1"))], &[("u__id", s("2"))]]),
        // Multiple rows (NOT OK), single column with HIDDEN result (OK)
        rows(&[
            &[("u__id", s("1")), ("score", Value::Int(10))],
            &[("u__id", s("2")), ("score", Value::Int(10))],
        ]),
        // Multiple row (NOT OK), multiple columns (NOT OK)
        rows(&[
            &[("u__id", s("1")), ("u__name", s("romanb"))],
            &[("u__id", s("2")), ("u__name", s("romanb"))],
        ]),
    ]
}

fn field_mapping() -> ResultSetMapping {
    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(CMS_USER, "u", None);
    rsm.add_field_result("u", "u__id", "id", None);
    rsm.add_field_result("u", "u__name", "name", None);
    rsm
}

fn scalar_mapping() -> ResultSetMapping {
    let mut rsm = ResultSetMapping::new();
    rsm.add_scalar_result("u__id", "id", "string");
    rsm.add_scalar_result("u__name", "name", "string");
    rsm
}

#[tokio::test]
async fn testHydrateSingleScalarFromFieldMappingWithValidResultSet() {
    let em = entity_manager();
    for (result_set, expected) in valid_result_set_provider() {
        let result = hydrate_all(&em, HYDRATE_SINGLE_SCALAR, result_set, &field_mapping())
            .await
            .unwrap();
        orm::assert_loose_eq(&expected, &result);
    }
}

#[tokio::test]
async fn testHydrateSingleScalarFromScalarMappingWithValidResultSet() {
    let em = entity_manager();
    for (result_set, expected) in valid_result_set_provider() {
        let result = hydrate_all(&em, HYDRATE_SINGLE_SCALAR, result_set, &scalar_mapping())
            .await
            .unwrap();
        orm::assert_loose_eq(&expected, &result);
    }
}

#[tokio::test]
async fn testHydrateSingleScalarFromFieldMappingWithInvalidResultSet() {
    let em = entity_manager();
    for result_set in invalid_result_set_provider() {
        let e = hydrate_all(&em, HYDRATE_SINGLE_SCALAR, result_set, &field_mapping())
            .await
            .unwrap_err();
        assert_eq!(e.class, NON_UNIQUE_RESULT_EXCEPTION);
    }
}

#[tokio::test]
async fn testHydrateSingleScalarFromScalarMappingWithInvalidResultSet() {
    let em = entity_manager();
    for result_set in invalid_result_set_provider() {
        let e = hydrate_all(&em, HYDRATE_SINGLE_SCALAR, result_set, &scalar_mapping())
            .await
            .unwrap_err();
        assert_eq!(e.class, NON_UNIQUE_RESULT_EXCEPTION);
    }
}
