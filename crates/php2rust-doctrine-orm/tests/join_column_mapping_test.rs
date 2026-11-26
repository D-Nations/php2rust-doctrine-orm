//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/JoinColumnMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::{JoinColumnMapping, serialize_object};

#[test]
fn testItSurvivesSerialization() {
    let mut mapping = JoinColumnMapping::new("foo", "id");

    mapping.deferrable = Some(true);
    mapping.unique = Some(true);
    mapping.quoted = Some(true);
    mapping.field_name = Some("bar".into());
    mapping.on_delete = Some("CASCADE".into());
    mapping.column_definition = Some("VARCHAR(255)".into());
    mapping.nullable = Some(true);
    mapping.referenced_column_name = "baz".into();
    mapping.options = Some(php_array!["foo" => "bar"]);
    mapping.foreign_key_name = Some("fk_foo_bar".into());

    let resurrected_mapping =
        JoinColumnMapping::unserialize(serialize_object(&mapping).as_bytes()).unwrap();

    assert_eq!(Some(true), resurrected_mapping.deferrable);
    assert_eq!("foo", resurrected_mapping.name);
    assert_eq!(Some(true), resurrected_mapping.unique);
    assert_eq!(Some(true), resurrected_mapping.quoted);
    assert_eq!(Some("bar"), resurrected_mapping.field_name.as_deref());
    assert_eq!(Some("CASCADE"), resurrected_mapping.on_delete.as_deref());
    assert_eq!(
        Some("VARCHAR(255)"),
        resurrected_mapping.column_definition.as_deref()
    );
    assert_eq!(Some(true), resurrected_mapping.nullable);
    assert_eq!("baz", resurrected_mapping.referenced_column_name);
    assert!(
        Value::Array(php_array!["foo" => "bar"])
            .identical(&Value::Array(resurrected_mapping.options.unwrap()))
    );
    assert_eq!(
        Some("fk_foo_bar"),
        resurrected_mapping.foreign_key_name.as_deref()
    );
}

#[test]
fn testFromMappingArrayIncludesForeignKeyName() {
    let mapping = JoinColumnMapping::from_mapping_array(&php_array![
        "name" => "user_id",
        "referencedColumnName" => "id",
        "foreignKeyName" => "fk_article_user",
    ])
    .unwrap();

    assert_eq!("user_id", mapping.name);
    assert_eq!("id", mapping.referenced_column_name);
    assert_eq!(Some("fk_article_user"), mapping.foreign_key_name.as_deref());
}
