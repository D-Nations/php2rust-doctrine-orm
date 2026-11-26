//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ManyToOneAssociationMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::mapping::{
    AssociationClass, AssociationKind, AssociationMapping, DefaultNamingStrategy,
    JoinColumnMapping, serialize_object,
};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\ManyToOneAssociationMappingTest";
const DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12126";

#[test]
fn testItSurvivesSerialization() {
    let class = AssociationClass::Final(AssociationKind::ManyToOne);
    let mut mapping = AssociationMapping::new(class, "foo", SELF, SELF);

    mapping.join_columns = vec![JoinColumnMapping::new("foo_id", "id")];
    mapping.join_column_field_names = php_array!["foo" => "bar"];
    mapping.source_to_target_key_columns = php_array!["foo" => "bar"];
    mapping.target_to_source_key_columns = php_array!["bar" => "foo"];

    let resurrected_mapping =
        AssociationMapping::unserialize(serialize_object(&mapping).as_bytes(), None).unwrap();

    assert_eq!(mapping.class, resurrected_mapping.class);
    assert_eq!(1, resurrected_mapping.join_columns.len());
    let same = |a: PhpArray, b: &PhpArray| Value::Array(a).identical(&Value::Array(b.clone()));
    assert!(same(
        php_array!["foo" => "bar"],
        &resurrected_mapping.join_column_field_names
    ));
    assert!(same(
        php_array!["foo" => "bar"],
        &resurrected_mapping.source_to_target_key_columns
    ));
    assert!(same(
        php_array!["bar" => "foo"],
        &resurrected_mapping.target_to_source_key_columns
    ));
}

/// `mappingsProvider()`.
fn mappings_provider() -> Vec<(&'static str, bool, bool, PhpArray)> {
    let mapping = |join_column: PhpArray, id: bool| {
        php_array![
            "fieldName" => "foo",
            "sourceEntity" => SELF,
            "targetEntity" => SELF,
            "isOwningSide" => true,
            "joinColumns" => PhpArray::list([Value::Array(join_column)]),
            "id" => id,
        ]
    };
    vec![
        (
            "not part of the identifier",
            false,
            true,
            mapping(
                php_array!["name" => "foo_id", "referencedColumnName" => "id"],
                false,
            ),
        ),
        (
            "part of the identifier",
            false,
            false,
            mapping(
                php_array!["name" => "foo_id", "referencedColumnName" => "id"],
                true,
            ),
        ),
        (
            "part of the identifier, but explicitly marked as nullable",
            true,
            false, // user's intent is ignored at the ORM level
            mapping(
                php_array!["name" => "foo_id", "referencedColumnName" => "id", "nullable" => true],
                true,
            ),
        ),
        (
            "part of the identifier, but explicitly marked as not nullable",
            true,
            false,
            mapping(
                php_array!["name" => "foo_id", "referencedColumnName" => "id", "nullable" => true],
                true,
            ),
        ),
    ]
}

#[test]
fn testNullableDefaults() {
    let naming_strategy = DefaultNamingStrategy::new();
    for (name, expect_deprecation, expected_value, mapping_array) in mappings_provider() {
        deprecations::enable_tracking_deprecations();
        let mapping = AssociationMapping::to_one_owning_side_from_mapping_array_and_name(
            AssociationKind::ManyToOne,
            &mapping_array,
            &naming_strategy,
            SELF,
            false,
            false,
        )
        .unwrap();

        for join_column in &mapping.join_columns {
            assert_eq!(Some(expected_value), join_column.nullable, "{name}");
        }
        assert_eq!(
            expect_deprecation,
            deprecations::count(DEPRECATION) > 0,
            "{name}"
        );
        deprecations::disable();
    }
}
