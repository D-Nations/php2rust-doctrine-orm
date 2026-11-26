//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/JoinTableMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{Key, Value, php_array};
use php2rust_doctrine_orm::mapping::{JoinColumnMapping, JoinTableMapping, serialize_object};

#[test]
fn testItSurvivesSerialization() {
    let mut mapping = JoinTableMapping::new("bar");

    mapping.quoted = Some(true);
    mapping.join_columns = vec![JoinColumnMapping::new("foo_id", "id")];
    mapping.inverse_join_columns = vec![JoinColumnMapping::new("bar_id", "id")];
    mapping.schema = Some("foo".into());
    mapping.options = php_array!["foo" => "bar"];

    let resurrected_mapping =
        JoinTableMapping::unserialize(serialize_object(&mapping).as_bytes()).unwrap();

    assert_eq!(Some(true), resurrected_mapping.quoted);
    assert_eq!(1, resurrected_mapping.join_columns.len());
    assert_eq!(1, resurrected_mapping.inverse_join_columns.len());
    assert_eq!(Some("foo"), resurrected_mapping.schema.as_deref());
    assert_eq!("bar", resurrected_mapping.name);
    assert!(
        Value::Array(php_array!["foo" => "bar"])
            .identical(&Value::Array(resurrected_mapping.options))
    );
}

#[test]
fn testConvertingItToAMappingArrayDoesNotContainNullableInformation() {
    let mut mapping = JoinTableMapping::new("bar");

    mapping.join_columns = vec![JoinColumnMapping::new("foo_id", "id")];
    mapping.inverse_join_columns = vec![JoinColumnMapping::new("bar_id", "id")];

    let mapping_array = mapping.to_array();

    let Some(Value::Array(join_columns)) = mapping_array.get(&Key::from("joinColumns")) else {
        panic!("joinColumns");
    };
    for join_column in join_columns.values() {
        let Value::Array(join_column) = join_column else {
            panic!("an array")
        };
        assert!(!join_column.contains_key(&Key::from("nullable")));
    }
}
