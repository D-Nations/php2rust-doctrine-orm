//! Port of `FieldBuilderTest`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/FieldBuilderTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::vectors::phpunit_equals;
use php_value::{Value, php_array};
use php2rust_doctrine_orm::mapping::builder::ClassMetadataBuilder;
use php2rust_doctrine_orm::mapping::class_metadata::{self as cm, ClassMetadata};

#[test]
fn testCustomIdGeneratorCanBeSet() {
    models::register();
    let cm_builder = ClassMetadataBuilder::new(Arc::new(ClassMetadata::new(
        "Doctrine\\Tests\\Models\\CMS\\CmsUser",
        None,
        None,
    )));

    let field_builder = cm_builder.create_field("aField", "string");

    let field_builder = field_builder.generated_value("CUSTOM");
    let field_builder = field_builder.set_custom_id_generator("stdClass");

    field_builder.build().unwrap();

    let metadata = cm_builder.get_class_metadata().read();
    assert_eq!(cm::GENERATOR_TYPE_CUSTOM, metadata.generator_type);
    assert!(phpunit_equals(
        &Value::Array(php_array! { "class" => "stdClass" }),
        &Value::Array(metadata.custom_generator_definition.clone().unwrap()),
    ));
}
