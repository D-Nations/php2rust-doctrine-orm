//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/NamingStrategyTest.php

#![allow(non_snake_case)]

#[path = "common/naming.rs"]
mod naming;

use naming::{CustomPascalNamingStrategy, JoinColumnClassNamingStrategy};
use php2rust_doctrine_orm::mapping::naming_strategy::{CASE_LOWER, CASE_UPPER};
use php2rust_doctrine_orm::mapping::{
    DefaultNamingStrategy, NamingStrategy, UnderscoreNamingStrategy,
};

type Strategy = Box<dyn NamingStrategy>;

fn default_naming() -> Strategy {
    Box::new(DefaultNamingStrategy::new())
}

fn underscore_naming_lower() -> Strategy {
    Box::new(UnderscoreNamingStrategy::new(CASE_LOWER))
}

fn underscore_naming_upper() -> Strategy {
    Box::new(UnderscoreNamingStrategy::new(CASE_UPPER))
}

fn custom_naming() -> Strategy {
    Box::new(CustomPascalNamingStrategy)
}

/// Data Provider for NamingStrategy#classToTableName
fn data_class_to_table_name() -> Vec<(Strategy, &'static str, &'static str)> {
    vec![
        // DefaultNamingStrategy
        (default_naming(), "SomeClassName", "SomeClassName"),
        (default_naming(), "SomeClassName", "\\SomeClassName"),
        (default_naming(), "Name", "\\Some\\Class\\Name"),
        (default_naming(), "Name2Test", "\\Some\\Class\\Name2Test"),
        // UnderscoreNamingStrategy
        (
            underscore_naming_lower(),
            "some_class_name",
            "\\Name\\Space\\SomeClassName",
        ),
        (underscore_naming_lower(), "name", "\\Some\\Class\\Name"),
        (
            underscore_naming_lower(),
            "name2_test",
            "\\Some\\Class\\Name2Test",
        ),
        (
            underscore_naming_lower(),
            "name2test",
            "\\Some\\Class\\Name2test",
        ),
        (
            underscore_naming_upper(),
            "SOME_CLASS_NAME",
            "\\Name\\Space\\SomeClassName",
        ),
        (underscore_naming_upper(), "NAME", "\\Some\\Class\\Name"),
        (
            underscore_naming_upper(),
            "NAME2_TEST",
            "\\Some\\Class\\Name2Test",
        ),
        (
            underscore_naming_upper(),
            "NAME2TEST",
            "\\Some\\Class\\Name2test",
        ),
        // CustomPascalNamingStrategy
        (custom_naming(), "SomeClassName", "SomeClassName"),
        (custom_naming(), "Name2Test", "\\Some\\Class\\Name2Test"),
    ]
}

#[test]
fn testClassToTableName() {
    for (strategy, expected, class_name) in data_class_to_table_name() {
        assert_eq!(expected, strategy.class_to_table_name(class_name));
    }
}

/// Data Provider for NamingStrategy#propertyToColumnName
fn data_property_to_column_name() -> Vec<(Strategy, &'static str, &'static str, &'static str)> {
    vec![
        // DefaultNamingStrategy
        (
            default_naming(),
            "someProperty",
            "someProperty",
            "Some\\Class",
        ),
        (
            default_naming(),
            "SOME_PROPERTY",
            "SOME_PROPERTY",
            "Some\\Class",
        ),
        (
            default_naming(),
            "some_property",
            "some_property",
            "Some\\Class",
        ),
        (
            default_naming(),
            "base64Encoded",
            "base64Encoded",
            "Some\\Class",
        ),
        (
            default_naming(),
            "base64_encoded",
            "base64_encoded",
            "Some\\Class",
        ),
        // UnderscoreNamingStrategy
        (
            underscore_naming_lower(),
            "some_property",
            "someProperty",
            "Some\\Class",
        ),
        (
            underscore_naming_lower(),
            "base64_encoded",
            "base64Encoded",
            "Some\\Class",
        ),
        (
            underscore_naming_lower(),
            "base64encoded",
            "base64encoded",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "SOME_PROPERTY",
            "someProperty",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "SOME_PROPERTY",
            "some_property",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "SOME_PROPERTY",
            "SOME_PROPERTY",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "BASE64_ENCODED",
            "base64Encoded",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "BASE64ENCODED",
            "base64encoded",
            "Some\\Class",
        ),
        // CustomPascalNamingStrategy
        (
            custom_naming(),
            "SomeProperty",
            "someProperty",
            "Some\\Class",
        ),
        (
            custom_naming(),
            "Base64Encoded",
            "base64Encoded",
            "Some\\Class",
        ),
    ]
}

#[test]
fn testPropertyToColumnName() {
    for (strategy, expected, property_name, class_name) in data_property_to_column_name() {
        assert_eq!(
            expected,
            strategy.property_to_column_name(property_name, class_name)
        );
    }
}

/// Data Provider for NamingStrategy#referenceColumnName
fn data_reference_column_name() -> Vec<(Strategy, &'static str)> {
    vec![
        // DefaultNamingStrategy
        (default_naming(), "id"),
        // UnderscoreNamingStrategy
        (underscore_naming_lower(), "id"),
        (underscore_naming_upper(), "ID"),
        // CustomPascalNamingStrategy
        (custom_naming(), "Id"),
    ]
}

#[test]
fn testReferenceColumnName() {
    for (strategy, expected) in data_reference_column_name() {
        assert_eq!(expected, strategy.reference_column_name());
    }
}

/// Data Provider for NamingStrategy#joinColumnName
fn data_join_column_name() -> Vec<(Strategy, &'static str, &'static str, &'static str)> {
    let join_column_class: fn() -> Strategy = || Box::new(JoinColumnClassNamingStrategy::default());
    vec![
        // DefaultNamingStrategy
        (
            default_naming(),
            "someColumn_id",
            "someColumn",
            "Some\\Class",
        ),
        (
            default_naming(),
            "some_column_id",
            "some_column",
            "Some\\Class",
        ),
        (
            default_naming(),
            "base64Encoded_id",
            "base64Encoded",
            "Some\\Class",
        ),
        (
            default_naming(),
            "base64_encoded_id",
            "base64_encoded",
            "Some\\Class",
        ),
        // UnderscoreNamingStrategy
        (
            underscore_naming_lower(),
            "some_column_id",
            "someColumn",
            "Some\\Class",
        ),
        (
            underscore_naming_lower(),
            "base64_encoded_id",
            "base64Encoded",
            "Some\\Class",
        ),
        (
            underscore_naming_lower(),
            "base64encoded_id",
            "base64encoded",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "SOME_COLUMN_ID",
            "someColumn",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "BASE64_ENCODED_ID",
            "base64Encoded",
            "Some\\Class",
        ),
        (
            underscore_naming_upper(),
            "BASE64ENCODED_ID",
            "base64encoded",
            "Some\\Class",
        ),
        // JoinColumnClassNamingStrategy
        (
            join_column_class(),
            "classname_someColumn_id",
            "someColumn",
            "Some\\ClassName",
        ),
        (
            join_column_class(),
            "classname_some_column_id",
            "some_column",
            "ClassName",
        ),
    ]
}

#[test]
fn testJoinColumnName() {
    for (strategy, expected, property_name, class_name) in data_join_column_name() {
        assert_eq!(
            expected,
            strategy.join_column_name(property_name, class_name)
        );
    }
}

/// Data Provider for NamingStrategy#joinTableName
fn data_join_table_name() -> Vec<(
    Strategy,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
)> {
    vec![
        // DefaultNamingStrategy
        (
            default_naming(),
            "someclassname_classname",
            "SomeClassName",
            "Some\\ClassName",
            "some_property",
        ),
        (
            default_naming(),
            "someclassname_classname",
            "\\SomeClassName",
            "ClassName",
            "some_property",
        ),
        (
            default_naming(),
            "name_classname",
            "\\Some\\Class\\Name",
            "ClassName",
            "some_property",
        ),
        // UnderscoreNamingStrategy
        (
            underscore_naming_lower(),
            "some_class_name_class_name",
            "SomeClassName",
            "Some\\ClassName",
            "some_property",
        ),
        (
            underscore_naming_lower(),
            "class1_test_class2_test",
            "Class1Test",
            "Some\\Class2Test",
            "some_property",
        ),
        (
            underscore_naming_lower(),
            "some_class_name_class_name",
            "\\SomeClassName",
            "ClassName",
            "some_property",
        ),
        (
            underscore_naming_lower(),
            "name_class_name",
            "\\Some\\Class\\Name",
            "ClassName",
            "some_property",
        ),
        (
            underscore_naming_upper(),
            "SOME_CLASS_NAME_CLASS_NAME",
            "SomeClassName",
            "Some\\ClassName",
            "some_property",
        ),
        (
            underscore_naming_upper(),
            "CLASS1_TEST_CLASS2_TEST",
            "Class1Test",
            "Some\\Class2Test",
            "some_property",
        ),
        (
            underscore_naming_upper(),
            "SOME_CLASS_NAME_CLASS_NAME",
            "\\SomeClassName",
            "ClassName",
            "some_property",
        ),
        (
            underscore_naming_upper(),
            "NAME_CLASS_NAME",
            "\\Some\\Class\\Name",
            "ClassName",
            "some_property",
        ),
    ]
}

#[test]
fn testJoinTableName() {
    for (strategy, expected, owner_entity, associated_entity, property_name) in
        data_join_table_name()
    {
        assert_eq!(
            expected,
            strategy.join_table_name(owner_entity, associated_entity, property_name)
        );
    }
}

/// Data Provider for NamingStrategy#joinKeyColumnName
fn data_join_key_column_name() -> Vec<(Strategy, &'static str, &'static str, Option<&'static str>)>
{
    vec![
        // DefaultNamingStrategy
        (default_naming(), "someclassname_id", "SomeClassName", None),
        (
            default_naming(),
            "name_identifier",
            "\\Some\\Class\\Name",
            Some("identifier"),
        ),
        // UnderscoreNamingStrategy
        (
            underscore_naming_lower(),
            "some_class_name2_test_id",
            "SomeClassName2Test",
            None,
        ),
        (
            underscore_naming_lower(),
            "some_class_name_id",
            "SomeClassName",
            None,
        ),
        (
            underscore_naming_lower(),
            "class_name_identifier",
            "\\Some\\Class\\ClassName",
            Some("identifier"),
        ),
        (
            underscore_naming_lower(),
            "name2_test_identifier",
            "\\Some\\Class\\Name2Test",
            Some("identifier"),
        ),
        (
            underscore_naming_upper(),
            "SOME_CLASS_NAME_ID",
            "SomeClassName",
            None,
        ),
        (
            underscore_naming_upper(),
            "SOME_CLASS_NAME2_TEST_ID",
            "SomeClassName2Test",
            None,
        ),
        (
            underscore_naming_upper(),
            "CLASS_NAME_IDENTIFIER",
            "\\Some\\Class\\ClassName",
            Some("IDENTIFIER"),
        ),
        (
            underscore_naming_upper(),
            "NAME2_TEST_IDENTIFIER",
            "\\Some\\Class\\Name2Test",
            Some("IDENTIFIER"),
        ),
    ]
}

#[test]
fn testJoinKeyColumnName() {
    for (strategy, expected, property_entity_name, referenced_column_name) in
        data_join_key_column_name()
    {
        assert_eq!(
            expected,
            strategy.join_key_column_name(property_entity_name, referenced_column_name)
        );
    }
}
