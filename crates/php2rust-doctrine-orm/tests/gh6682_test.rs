//! Ticket/GH6682Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6682Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::class_metadata::ClassMetadataInfo;

#[test]
fn testIssue() {
    let parsed_definition = php_array![
        "sequenceName" => "test_sequence",
        "allocationSize" => "",
        "initialValue" => "",
    ];

    let mut class_metadata = ClassMetadataInfo::new("test_entity", None, None);
    class_metadata
        .set_sequence_generator_definition(parsed_definition)
        .unwrap();

    assert!(
        Value::Array(php_array![
            "sequenceName" => "test_sequence",
            "allocationSize" => "1",
            "initialValue" => "1",
        ])
        .identical(&Value::Array(
            class_metadata.sequence_generator_definition.clone().unwrap()
        ))
    );
}
