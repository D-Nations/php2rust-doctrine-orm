//! Port of `TypedFieldMapperTest`, on the recorded test models
//! (tests/common/models.rs), with the test's
//! `CustomIntAsStringTypedFieldMapper`. `assertSame` on the arrays
//! compares the keys in order and the values strictly.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/TypedFieldMapperTest.php tests/Tests/ORM/Mapping/TypedFieldMapper/CustomIntAsStringTypedFieldMapper.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value, php_array};
use php2rust_doctrine_orm::mapping::typed_field_mapper::{
    ChainTypedFieldMapper, DefaultTypedFieldMapper, TypedFieldMapper,
};
use php2rust_doctrine_persistence::php_reflection::ReflectionProperty;

const USER_TYPED: &str = "Doctrine\\Tests\\Models\\TypedProperties\\UserTyped";

/// `Doctrine\Tests\ORM\Mapping\TypedFieldMapper\CustomIntAsStringTypedFieldMapper`.
#[derive(Debug)]
struct CustomIntAsStringTypedFieldMapper;

impl TypedFieldMapper for CustomIntAsStringTypedFieldMapper {
    fn validate_and_complete(
        &self,
        mut mapping: PhpArray,
        field: &ReflectionProperty,
    ) -> Result<PhpArray, Thrown> {
        let ty = field.get_type();

        if mapping.get(&"type".into()).is_none()
            && let Some(ty) = ty
            && ty.get_name() == "int"
        {
            mapping.insert("type", Value::from("string"));
        }

        Ok(mapping)
    }
}

fn default_typed_field_mapper() -> Arc<dyn TypedFieldMapper> {
    Arc::new(DefaultTypedFieldMapper::default())
}

fn custom_typed_field_mapper() -> Arc<dyn TypedFieldMapper> {
    Arc::new(CustomIntAsStringTypedFieldMapper)
}

fn chain_typed_field_mapper() -> Arc<dyn TypedFieldMapper> {
    Arc::new(ChainTypedFieldMapper::new(vec![
        custom_typed_field_mapper(),
        default_typed_field_mapper(),
    ]))
}

/// `dataFieldToMappedField()`: the mapper, the field and its type.
fn data_field_to_mapped_field() -> Vec<(Arc<dyn TypedFieldMapper>, &'static str, &'static str)> {
    vec![
        // DefaultTypedFieldMapper
        (default_typed_field_mapper(), "id", "integer"),
        (default_typed_field_mapper(), "username", "string"),
        (default_typed_field_mapper(), "dateInterval", "dateinterval"),
        (default_typed_field_mapper(), "dateTime", "datetime"),
        (
            default_typed_field_mapper(),
            "dateTimeImmutable",
            "datetime_immutable",
        ),
        (default_typed_field_mapper(), "array", "json"),
        (default_typed_field_mapper(), "boolean", "boolean"),
        (default_typed_field_mapper(), "float", "float"),
        (default_typed_field_mapper(), "bodyHeight", "number"),
        // CustomIntAsStringTypedFieldMapper
        (custom_typed_field_mapper(), "id", "string"),
        // ChainTypedFieldMapper
        (chain_typed_field_mapper(), "id", "string"),
        (chain_typed_field_mapper(), "username", "string"),
    ]
}

fn entries(array: &PhpArray) -> Vec<(String, Value)> {
    array
        .iter()
        .map(|(key, value)| (format!("{key:?}"), value.clone()))
        .collect()
}

#[test]
fn testValidateAndComplete() {
    models::register();
    for (index, (typed_field_mapper, field_name, ty)) in
        data_field_to_mapped_field().into_iter().enumerate()
    {
        let mapping = php_array! { "fieldName" => field_name };
        let final_mapping = php_array! { "fieldName" => field_name, "type" => ty };
        let actual = typed_field_mapper
            .validate_and_complete(
                mapping,
                &ReflectionProperty::new(USER_TYPED, field_name).unwrap(),
            )
            .unwrap();
        assert_eq!(
            entries(&final_mapping),
            entries(&actual),
            "data set #{index}"
        );
    }
}
