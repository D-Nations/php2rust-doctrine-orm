//! ParameterTypeInfererTest.php at doctrine/orm
//! 00c2fd70da291cca1ec75f4dae5344fe3df588fa. The provider's rows run in
//! one test, labelled. The enum cases of tests/Tests/Models/Enums are
//! `EnumCase` objects backed as upstream declares them
//! (`AccessLevel::Admin = 1`, `UserStatus::Active = 'active'`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/ParameterTypeInfererTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use std::sync::Arc;

use php_value::date_interval::DateInterval;
use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::object::EnumCase;
use php_value::{PhpArray, Value};
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_dbal::{ArrayParameterType, ParameterType, WrapperParameterType};
use php2rust_doctrine_orm::query::ParameterTypeInferer;

fn access_level_admin() -> Value {
    Value::Object(Arc::new(EnumCase::new(
        "Doctrine\\Tests\\Models\\Enums\\AccessLevel",
        "Admin",
        Some(Value::Int(ACCESS_LEVEL_ADMIN)),
    )))
}

fn user_status_active() -> Value {
    Value::Object(Arc::new(EnumCase::new(
        "Doctrine\\Tests\\Models\\Enums\\UserStatus",
        "Active",
        Some(Value::from(USER_STATUS_ACTIVE)),
    )))
}

const ACCESS_LEVEL_ADMIN: i64 = 1;
const USER_STATUS_ACTIVE: &str = "active";

fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

fn provider_parameter_type_inferer() -> Vec<(&'static str, Value, WrapperParameterType)> {
    let now = DateTimeImmutable::now(TimeZone::Utc);
    vec![
        ("integer", Value::Int(1), Types::INTEGER.into()),
        ("string", Value::from("bar"), ParameterType::String.into()),
        (
            "numeric_string",
            Value::from("1"),
            ParameterType::String.into(),
        ),
        (
            "datetime_object",
            DateTime::object(now),
            Types::DATETIME_MUTABLE.into(),
        ),
        (
            "datetime_immutable_object",
            Value::Object(Arc::new(now)),
            Types::DATETIME_IMMUTABLE.into(),
        ),
        (
            "date_interval_object",
            Value::Object(Arc::new(DateInterval::new(b"P1D").unwrap().unwrap())),
            Types::DATEINTERVAL.into(),
        ),
        (
            "array_of_int",
            list(vec![Value::Int(2)]),
            ArrayParameterType::Integer.into(),
        ),
        (
            "array_of_string",
            list(vec![Value::from("foo")]),
            ArrayParameterType::String.into(),
        ),
        (
            "array_of_numeric_string",
            list(vec![Value::from("1"), Value::from("2")]),
            ArrayParameterType::String.into(),
        ),
        (
            "empty_array",
            list(vec![]),
            ArrayParameterType::String.into(),
        ),
        ("boolean", Value::Bool(true), Types::BOOLEAN.into()),
        (
            "int_backed_enum",
            access_level_admin(),
            Types::INTEGER.into(),
        ),
        (
            "string_backed_enum",
            user_status_active(),
            Types::STRING.into(),
        ),
        (
            "array_of_int_backed_enum",
            list(vec![access_level_admin()]),
            ArrayParameterType::Integer.into(),
        ),
        (
            "array_of_string_backed_enum",
            list(vec![user_status_active()]),
            ArrayParameterType::String.into(),
        ),
    ]
}

#[test]
fn testParameterTypeInferer() {
    for (label, value, expected) in provider_parameter_type_inferer() {
        assert_eq!(
            expected,
            ParameterTypeInferer::infer_type(&value),
            "{label}"
        );
    }
}
