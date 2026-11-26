//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/DiscriminatorColumnMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::{DiscriminatorColumnMapping, serialize_object};

#[test]
fn testItSurvivesSerialization() {
    let mut mapping = DiscriminatorColumnMapping::new("string", "discr", "discr");

    mapping.length = Some(255);
    mapping.column_definition = Some("VARCHAR(255)".into());
    mapping.enum_type = Some("MyEnum".into());
    mapping.options = php_array!["foo" => "bar"];

    let resurrected_mapping =
        DiscriminatorColumnMapping::unserialize(serialize_object(&mapping).as_bytes()).unwrap();

    assert_eq!(Some(255), resurrected_mapping.length);
    assert_eq!(
        Some("VARCHAR(255)"),
        resurrected_mapping.column_definition.as_deref()
    );
    assert_eq!(Some("MyEnum"), resurrected_mapping.enum_type.as_deref());
    assert!(
        Value::Array(php_array!["foo" => "bar"])
            .identical(&Value::Array(resurrected_mapping.options))
    );
}
