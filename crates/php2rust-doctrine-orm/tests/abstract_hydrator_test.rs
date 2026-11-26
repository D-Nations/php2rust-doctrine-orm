//! Upstream: doctrine/orm tests/Tests/ORM/Hydration/AbstractHydratorTest.php
//!
//! `buildEnumForTesting()` is [`build_enum`]. The `onClear` listener tests
//! are not ported: the hydrators read a result whole and do not register
//! the listener.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::internal::hydration::build_enum;
use php2rust_doctrine_persistence::php_reflection;

const ACCESS_LEVEL: &str = "Doctrine\\Tests\\Models\\Enums\\AccessLevel";
const USER_STATUS: &str = "Doctrine\\Tests\\Models\\Enums\\UserStatus";

/// `$enum::Case`.
fn case(enum_type: &str, name: &str) -> Value {
    models::register();
    let class = php_reflection::class_info(enum_type).expect("the enum is declared");
    let case = class
        .cases()
        .iter()
        .find(|case| {
            case.property(b"name")
                .is_some_and(|n| n == Value::string(name))
        })
        .cloned()
        .expect("the case is declared");
    Value::Object(case)
}

fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

#[test]
fn testEnumCastsIntegerBackedEnumValues() {
    models::register();
    let access_level = build_enum(&Value::string("2"), ACCESS_LEVEL).unwrap();
    let user_status = build_enum(&Value::string("active"), USER_STATUS).unwrap();

    assert!(case(ACCESS_LEVEL, "User").identical(&access_level));
    assert!(case(USER_STATUS, "Active").identical(&user_status));
}

#[test]
fn testEnumCastsIntegerBackedEnumArrayValues() {
    models::register();
    let access_levels = build_enum(
        &list(vec![Value::string("1"), Value::string("2")]),
        ACCESS_LEVEL,
    )
    .unwrap();
    let user_status = build_enum(
        &list(vec![Value::string("active"), Value::string("inactive")]),
        USER_STATUS,
    )
    .unwrap();

    assert!(
        list(vec![
            case(ACCESS_LEVEL, "Admin"),
            case(ACCESS_LEVEL, "User")
        ])
        .identical(&access_levels)
    );
    assert!(
        list(vec![
            case(USER_STATUS, "Active"),
            case(USER_STATUS, "Inactive")
        ])
        .identical(&user_status)
    );
}
