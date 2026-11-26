//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ManyToManyOwningSideMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::mapping::{
    AssociationClass, AssociationKind, AssociationMapping, DefaultNamingStrategy, JoinTableMapping,
    serialize_object,
};

const SELF: &str = "Doctrine\\Tests\\ORM\\Mapping\\ManyToManyOwningSideMappingTest";
const DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12126";

#[test]
fn testItSurvivesSerialization() {
    let class = AssociationClass::Final(AssociationKind::ManyToManyOwningSide);
    let mut mapping = AssociationMapping::new(class, "foo", SELF, SELF);

    mapping.join_table = Some(JoinTableMapping::new("bar"));
    mapping.join_table_columns = vec!["foo".into(), "bar".into()];
    mapping.relation_to_source_key_columns = php_array!["foo" => "bar"];
    mapping.relation_to_target_key_columns = php_array!["bar" => "baz"];

    let resurrected_mapping =
        AssociationMapping::unserialize(serialize_object(&mapping).as_bytes(), None).unwrap();

    assert!(resurrected_mapping.is_many_to_many_owning_side());
    assert_eq!("bar", resurrected_mapping.join_table.as_ref().unwrap().name);
    assert_eq!(
        vec!["foo".to_owned(), "bar".to_owned()],
        resurrected_mapping.join_table_columns
    );
    let same = |a: PhpArray, b: &PhpArray| Value::Array(a).identical(&Value::Array(b.clone()));
    assert!(same(
        php_array!["foo" => "bar"],
        &resurrected_mapping.relation_to_source_key_columns
    ));
    assert!(same(
        php_array!["bar" => "baz"],
        &resurrected_mapping.relation_to_target_key_columns
    ));
}

/// `mappingsProvider()`.
fn mappings_provider() -> Vec<(&'static str, bool, bool, PhpArray)> {
    let mapping = |join_column: PhpArray, inverse_join_column: PhpArray, id: Option<bool>| {
        let mut mapping = php_array![
            "fieldName" => "foo",
            "sourceEntity" => SELF,
            "targetEntity" => SELF,
            "isOwningSide" => true,
            "joinTable" => php_array![
                "name" => "bar",
                "joinColumns" => PhpArray::list([Value::Array(join_column)]),
                "inverseJoinColumns" => PhpArray::list([Value::Array(inverse_join_column)]),
            ],
        ];
        if let Some(id) = id {
            mapping.insert("id", Value::Bool(id));
        }
        mapping
    };
    let bar_id = || php_array!["name" => "bar_id", "referencedColumnName" => "id"];
    let foo_id = || php_array!["name" => "foo_id", "referencedColumnName" => "id"];
    vec![
        (
            "defaults to false",
            false,
            false,
            mapping(bar_id(), foo_id(), None),
        ),
        (
            "explicitly marked as nullable",
            true,
            false, // user's intent is ignored at the ORM level
            mapping(
                php_array!["name" => "bar_id", "referencedColumnName" => "id", "nullable" => true],
                foo_id(),
                Some(true),
            ),
        ),
        (
            "explicitly marked as nullable (inverse column)",
            true,
            false, // user's intent is ignored at the ORM level
            mapping(
                bar_id(),
                php_array!["name" => "foo_id", "referencedColumnName" => "id", "nullable" => true],
                Some(true),
            ),
        ),
        (
            "explicitly marked as not nullable",
            true,
            false,
            mapping(
                php_array!["name" => "bar_id", "referencedColumnName" => "id", "nullable" => false],
                foo_id(),
                Some(true),
            ),
        ),
        (
            "explicitly marked as not nullable (inverse column)",
            true,
            false,
            mapping(
                bar_id(),
                php_array!["name" => "foo_id", "referencedColumnName" => "id", "nullable" => false],
                Some(true),
            ),
        ),
    ]
}

#[test]
fn testNullableDefaults() {
    let naming_strategy = DefaultNamingStrategy::new();
    for (name, expect_deprecation, expected_value, mapping_array) in mappings_provider() {
        deprecations::enable_tracking_deprecations();
        let mapping =
            AssociationMapping::many_to_many_owning_side_from_mapping_array_and_naming_strategy(
                &mapping_array,
                &naming_strategy,
            )
            .unwrap();

        for join_column in &mapping.join_table.as_ref().unwrap().join_columns {
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
