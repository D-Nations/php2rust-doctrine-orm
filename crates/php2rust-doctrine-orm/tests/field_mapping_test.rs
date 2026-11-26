//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/FieldMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::{FieldMapping, serialize_object};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\FieldMappingTest";
/// `ClassMetadata::GENERATOR_TYPE_AUTO`.
const GENERATOR_TYPE_AUTO: i64 = 1;

#[test]
fn testItSurvivesSerialization() {
    let mut mapping = FieldMapping::new("string", "id", "id");

    mapping.length = Some(255);
    mapping.id = Some(true);
    mapping.id_position = Some(2);
    mapping.nullable = Some(true);
    mapping.not_insertable = Some(true);
    mapping.not_updatable = Some(true);
    mapping.column_definition = Some("VARCHAR(255)".into());
    mapping.generated = Some(GENERATOR_TYPE_AUTO);
    mapping.enum_type = Some("MyEnum".into());
    mapping.precision = Some(10);
    mapping.scale = Some(2);
    mapping.unique = Some(true);
    mapping.index = Some(true);
    mapping.inherited = Some(SELF.into());
    mapping.original_class = Some(SELF.into());
    mapping.original_field = Some("id".into());
    mapping.quoted = Some(true);
    mapping.declared = Some(SELF.into());
    mapping.declared_field = Some("id".into());
    mapping.options = Some(php_array!["foo" => "bar"]);
    mapping.version = Some(true);
    mapping.default = Value::from("foo");

    let resurrected_mapping =
        FieldMapping::unserialize(serialize_object(&mapping).as_bytes()).unwrap();

    assert_eq!(Some(255), resurrected_mapping.length);
    assert_eq!(Some(true), resurrected_mapping.id);
    assert_eq!(Some(2), resurrected_mapping.id_position);
    assert_eq!(Some(true), resurrected_mapping.nullable);
    assert_eq!(Some(true), resurrected_mapping.not_insertable);
    assert_eq!(Some(true), resurrected_mapping.not_updatable);
    assert_eq!(
        Some("VARCHAR(255)"),
        resurrected_mapping.column_definition.as_deref()
    );
    assert_eq!(Some(GENERATOR_TYPE_AUTO), resurrected_mapping.generated);
    assert_eq!(Some("MyEnum"), resurrected_mapping.enum_type.as_deref());
    assert_eq!(Some(10), resurrected_mapping.precision);
    assert_eq!(Some(2), resurrected_mapping.scale);
    assert_eq!(Some(true), resurrected_mapping.unique);
    assert_eq!(Some(true), resurrected_mapping.index);
    assert_eq!(Some(SELF), resurrected_mapping.inherited.as_deref());
    assert_eq!(Some(SELF), resurrected_mapping.original_class.as_deref());
    assert_eq!(Some("id"), resurrected_mapping.original_field.as_deref());
    assert_eq!(Some(true), resurrected_mapping.quoted);
    assert_eq!(Some(SELF), resurrected_mapping.declared.as_deref());
    assert_eq!(Some("id"), resurrected_mapping.declared_field.as_deref());
    assert!(
        Value::Array(php_array!["foo" => "bar"])
            .identical(&Value::Array(resurrected_mapping.options.clone().unwrap()))
    );
    assert_eq!(Some(true), resurrected_mapping.version);
    assert!(Value::from("foo").identical(&resurrected_mapping.default));
}
