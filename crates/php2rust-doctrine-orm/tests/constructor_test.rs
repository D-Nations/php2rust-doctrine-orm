//! Port of `Entity\ConstructorTest`: the constructor of
//! `ConstructorTestEntity1` is written here, on its recorded class.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Entity/ConstructorTest.php

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

use php_value::Object;

use orm::{get, new_object, s, set};

/// `new ConstructorTestEntity1($username)`.
fn constructor_test_entity1(username: Option<&str>) -> Object {
    let entity = new_object("Doctrine\\Tests\\ORM\\Entity\\ConstructorTestEntity1");
    if let Some(username) = username {
        set(&entity, "username", s(username));
    }
    entity
}

#[test]
fn testFieldInitializationInConstructor() {
    let entity = constructor_test_entity1(Some("romanb"));
    orm::assert_loose_eq(&s("romanb"), &get(&entity, "username"));
}
