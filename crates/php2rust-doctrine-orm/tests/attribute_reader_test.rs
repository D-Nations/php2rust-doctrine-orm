//! Port of `AttributeReaderTest`: the reader's repeatable checks and the
//! `options` of `JoinTable`, `JoinColumn`, `InverseJoinColumn` and
//! `DiscriminatorColumn`, on the test's models (tests/common/models.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/AttributeReaderTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_orm::mapping::attributes::{self as attr, AttributeClass};
use php2rust_doctrine_orm::mapping::driver::AttributeReader;
use php2rust_doctrine_persistence::php_reflection::{ReflectionClass, ReflectionProperty};

const NS: &str = "Doctrine\\Tests\\ORM\\Mapping\\";

fn property(class: &str, name: &str) -> ReflectionProperty {
    models::register();
    ReflectionProperty::new(&format!("{NS}{class}"), name).unwrap()
}

fn options(charset: &str, collation: &str) -> PhpArray {
    php_array!["charset" => charset, "collation" => collation]
}

#[test]
fn testItThrowsWhenGettingRepeatableAttributeWithTheWrongMethod() {
    let reader = AttributeReader::new();
    let property = property("TestEntity", "id");
    let error = reader
        .get_property_attribute::<attr::Index>(&property)
        .unwrap_err();
    assert_eq!(error.class, "LogicException");
    assert_eq!(
        error.message,
        "The attribute \"Doctrine\\ORM\\Mapping\\Index\" is repeatable. Call getPropertyAttributeCollection() instead."
    );
}

#[test]
fn testItThrowsWhenGettingNonRepeatableAttributeWithTheWrongMethod() {
    let reader = AttributeReader::new();
    let property = property("TestEntity", "id");
    let error = reader
        .get_property_attribute_collection::<attr::Id>(&property)
        .unwrap_err();
    assert_eq!(error.class, "LogicException");
    assert_eq!(
        error.message,
        "The attribute \"Doctrine\\ORM\\Mapping\\Id\" is not repeatable. Call getPropertyAttribute() instead."
    );
}

#[test]
fn testJoinTableOptions() {
    let reader = AttributeReader::new();
    let property = property("TestEntity", "tags");
    let join_table = reader
        .get_property_attribute::<attr::JoinTable>(&property)
        .unwrap()
        .unwrap();
    assert!(
        Value::Array(join_table.options)
            .identical(&Value::Array(options("ascii", "ascii_general_ci")))
    );
}

#[test]
fn testJoinColumnOptions() {
    let reader = AttributeReader::new();
    let property = property("TestEntity", "tags");
    let join_columns = reader
        .get_property_attribute_collection::<attr::JoinColumn>(&property)
        .unwrap();
    assert_eq!(join_columns.len(), 1);
    assert!(
        Value::Array(join_columns[0].options.clone())
            .identical(&Value::Array(options("latin1", "latin1_swedish_ci")))
    );
    let inverse_join_columns = reader
        .get_property_attribute_collection::<attr::InverseJoinColumn>(&property)
        .unwrap();
    assert_eq!(inverse_join_columns.len(), 1);
    assert!(
        Value::Array(inverse_join_columns[0].options.clone())
            .identical(&Value::Array(options("utf8mb4", "utf8mb4_bin")))
    );
}

#[test]
fn testDiscriminatedColumnOptions() {
    models::register();
    let reader = AttributeReader::new();
    let class = ReflectionClass::new(&format!("{NS}TestPerson")).unwrap();
    let attributes = reader.get_class_attributes(&class);
    assert!(attributes.contains(attr::DiscriminatorColumn::CLASS));
    let column = attributes.get::<attr::DiscriminatorColumn>().unwrap();
    assert!(
        Value::Array(column.options.clone())
            .identical(&Value::Array(options("ascii", "ascii_general_ci")))
    );
}
