//! Port of `ReflectionReadonlyPropertyTest`, on the recorded test models
//! (tests/common/models.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ReflectionReadonlyPropertyTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::{Object, Value};
use php2rust_doctrine_orm::mapping::ReflectionReadonlyProperty;
use php2rust_doctrine_orm::object::EntityObject;
use php2rust_doctrine_persistence::php_reflection::ReflectionProperty;

const AUTHOR: &str = "Doctrine\\Tests\\Models\\ReadonlyProperties\\Author";

fn author() -> Object {
    models::register();
    EntityObject::object(AUTHOR).unwrap()
}

#[test]
fn testSecondWriteWithSameValue() {
    let author = author();

    let wrapped_reflection = ReflectionProperty::new(AUTHOR, "name").unwrap();
    let reflection = ReflectionReadonlyProperty::new(wrapped_reflection.clone()).unwrap();

    reflection
        .set_value(author.as_ref(), Value::from("John Doe"))
        .unwrap();

    assert_eq!(
        Value::from("John Doe"),
        wrapped_reflection.get_value(author.as_ref()).unwrap()
    );
    assert_eq!(
        Value::from("John Doe"),
        reflection.get_value(author.as_ref()).unwrap()
    );

    reflection
        .set_value(author.as_ref(), Value::from("John Doe"))
        .unwrap();

    assert_eq!(
        Value::from("John Doe"),
        wrapped_reflection.get_value(author.as_ref()).unwrap()
    );
    assert_eq!(
        Value::from("John Doe"),
        reflection.get_value(author.as_ref()).unwrap()
    );
}

#[test]
fn testSecondWriteWithDifferentValue() {
    let author = author();

    let wrapped_reflection = ReflectionProperty::new(AUTHOR, "name").unwrap();
    let reflection = ReflectionReadonlyProperty::new(wrapped_reflection).unwrap();

    reflection
        .set_value(author.as_ref(), Value::from("John Doe"))
        .unwrap();

    let error = reflection
        .set_value(author.as_ref(), Value::from("Jane Doe"))
        .unwrap_err();
    assert_eq!("LogicException", error.class);
    assert_eq!(
        "Attempting to change readonly property Doctrine\\Tests\\Models\\ReadonlyProperties\\Author::$name.",
        String::from_utf8_lossy(error.message.as_bytes())
    );
}

#[test]
fn testNonReadonlyPropertiesAreForbidden() {
    models::register();
    let reflection =
        ReflectionProperty::new("Doctrine\\Tests\\Models\\CMS\\CmsTag", "name").unwrap();

    let error = ReflectionReadonlyProperty::new(reflection).unwrap_err();
    assert_eq!("InvalidArgumentException", error.class);
}
