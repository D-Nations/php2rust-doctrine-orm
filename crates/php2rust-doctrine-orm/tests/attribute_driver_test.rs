//! Port of `AttributeDriverTest`, and of the `MappingDriverTestCase` tests
//! it inherits (in `common/mapping_driver_test_case.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/AttributeDriverTest.php tests/Tests/ORM/Mapping/MappingDriverTestCase.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/mapping_driver_test_case.rs"]
mod mapping_driver_test_case;
#[path = "common/models.rs"]
mod models;
#[path = "common/naming.rs"]
mod naming;

use std::sync::Arc;

use php_value::vectors::phpunit_equals;
use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_orm::mapping::JoinColumnMapping;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;
use php2rust_doctrine_persistence::mapping::driver::{ClassNames, MappingDriver};

use mapping_driver_test_case::{assert_array, factory, model, ns};

/// `AttributeDriver`.
fn attribute_driver() -> AttributeDriver {
    models::register();
    AttributeDriver::new(Vec::<String>::new(), true).unwrap()
}

/// Uses the XML driver (`$this->loadDriver() instanceof XmlDriver`).
pub const USES_XML_DRIVER: bool = false;

/// `loadDriver()`.
pub fn load_driver() -> Arc<dyn MappingDriver> {
    Arc::new(attribute_driver())
}

// MappingDriverTestCase

#[test]
fn testEntityTableNameAndInheritance() {
    mapping_driver_test_case::testEntityTableNameAndInheritance();
}

#[test]
fn testEntityIndexes() {
    mapping_driver_test_case::testEntityIndexes();
}

#[test]
fn testEntityIncorrectIndexes() {
    mapping_driver_test_case::testEntityIncorrectIndexes();
}

#[test]
fn testEntityIndexFlagsAndPartialIndexes() {
    mapping_driver_test_case::testEntityIndexFlagsAndPartialIndexes();
}

#[test]
fn testEntityUniqueConstraints() {
    mapping_driver_test_case::testEntityUniqueConstraints();
}

#[test]
fn testEntityIncorrectUniqueContraint() {
    mapping_driver_test_case::testEntityIncorrectUniqueContraint();
}

#[test]
fn testEntityOptions() {
    mapping_driver_test_case::testEntityOptions();
}

#[test]
fn testEntitySequence() {
    mapping_driver_test_case::testEntitySequence();
}

#[test]
fn testEntityCustomGenerator() {
    mapping_driver_test_case::testEntityCustomGenerator();
}

#[test]
fn testFieldMappings() {
    mapping_driver_test_case::testFieldMappings();
}

#[test]
fn testVersionedField() {
    mapping_driver_test_case::testVersionedField();
}

#[test]
fn testFieldMappingsColumnNames() {
    mapping_driver_test_case::testFieldMappingsColumnNames();
}

#[test]
fn testStringFieldMappings() {
    mapping_driver_test_case::testStringFieldMappings();
}

#[test]
fn testFieldTypeFromReflection() {
    mapping_driver_test_case::testFieldTypeFromReflection();
}

#[test]
fn testCustomFieldTypeFromReflection() {
    mapping_driver_test_case::testCustomFieldTypeFromReflection();
}

#[test]
fn testFieldOptions() {
    mapping_driver_test_case::testFieldOptions();
}

#[test]
fn testIdFieldOptions() {
    mapping_driver_test_case::testIdFieldOptions();
}

#[test]
fn testIdentifier() {
    mapping_driver_test_case::testIdentifier();
}

#[test]
fn testBooleanValuesForOptionIsSetCorrectly() {
    mapping_driver_test_case::testBooleanValuesForOptionIsSetCorrectly();
}

#[test]
fn testAssociations() {
    mapping_driver_test_case::testAssociations();
}

#[test]
fn testOwningOneToOneAssociation() {
    mapping_driver_test_case::testOwningOneToOneAssociation();
}

#[test]
fn testInverseOneToManyAssociation() {
    mapping_driver_test_case::testInverseOneToManyAssociation();
}

#[test]
fn testOrderByWithStringValues() {
    mapping_driver_test_case::testOrderByWithStringValues();
}

#[test]
fn testOrderByWithInvalidDirection() {
    mapping_driver_test_case::testOrderByWithInvalidDirection();
}

#[test]
fn testManyToManyAssociationWithCascadeAll() {
    mapping_driver_test_case::testManyToManyAssociationWithCascadeAll();
}

#[test]
fn testLifecycleCallbacks() {
    mapping_driver_test_case::testLifecycleCallbacks();
}

#[test]
fn testLifecycleCallbacksSupportMultipleMethodNames() {
    mapping_driver_test_case::testLifecycleCallbacksSupportMultipleMethodNames();
}

#[test]
fn testJoinColumnUniqueAndNullable() {
    mapping_driver_test_case::testJoinColumnUniqueAndNullable();
}

#[test]
fn testColumnDefinition() {
    mapping_driver_test_case::testColumnDefinition();
}

#[test]
fn testJoinColumnOnDelete() {
    mapping_driver_test_case::testJoinColumnOnDelete();
}

#[test]
fn testDiscriminatorColumnDefaults() {
    mapping_driver_test_case::testDiscriminatorColumnDefaults();
}

#[test]
fn testDefaultFieldType() {
    mapping_driver_test_case::testDefaultFieldType();
}

#[test]
fn testIdentifierColumnDefinition() {
    mapping_driver_test_case::testIdentifierColumnDefinition();
}

#[test]
fn testNamingStrategy() {
    mapping_driver_test_case::testNamingStrategy();
}

#[test]
fn testDiscriminatorColumnDefinition() {
    mapping_driver_test_case::testDiscriminatorColumnDefinition();
}

#[test]
fn testDiscriminatorColumnEnumTypeDefinition() {
    mapping_driver_test_case::testDiscriminatorColumnEnumTypeDefinition();
}

#[test]
fn testInvalidEntityOrMappedSuperClassShouldMentionParentClasses() {
    mapping_driver_test_case::testInvalidEntityOrMappedSuperClassShouldMentionParentClasses();
}

#[test]
fn testIdentifierRequiredShouldMentionParentClasses() {
    mapping_driver_test_case::testIdentifierRequiredShouldMentionParentClasses();
}

#[test]
fn testInversedByOverrideMapping() {
    mapping_driver_test_case::testInversedByOverrideMapping();
}

#[test]
fn testFetchOverrideMapping() {
    mapping_driver_test_case::testFetchOverrideMapping();
}

#[test]
fn testAssociationOverridesMapping() {
    mapping_driver_test_case::testAssociationOverridesMapping();
}

#[test]
fn testAttributeOverridesMapping() {
    mapping_driver_test_case::testAttributeOverridesMapping();
}

#[test]
fn testEntityListeners() {
    mapping_driver_test_case::testEntityListeners();
}

#[test]
fn testEntityListenersOverride() {
    mapping_driver_test_case::testEntityListenersOverride();
}

#[test]
fn testEntityListenersNamingConvention() {
    mapping_driver_test_case::testEntityListenersNamingConvention();
}

#[test]
fn testSecondLevelCacheMapping() {
    mapping_driver_test_case::testSecondLevelCacheMapping();
}

#[test]
fn testSchemaDefinitionViaExplicitTableSchemaAttributeProperty() {
    mapping_driver_test_case::testSchemaDefinitionViaExplicitTableSchemaAttributeProperty();
}

#[test]
fn testSchemaDefinitionViaSchemaDefinedInTableNameInTableAttributeProperty() {
    mapping_driver_test_case::testSchemaDefinitionViaSchemaDefinedInTableNameInTableAttributeProperty();
}

#[test]
fn testDiscriminatorColumnDefaultLength() {
    mapping_driver_test_case::testDiscriminatorColumnDefaultLength();
}

#[test]
fn testDiscriminatorColumnDefaultType() {
    mapping_driver_test_case::testDiscriminatorColumnDefaultType();
}

#[test]
fn testDiscriminatorColumnDefaultName() {
    mapping_driver_test_case::testDiscriminatorColumnDefaultName();
}

#[test]
fn testReservedWordInTableColumn() {
    mapping_driver_test_case::testReservedWordInTableColumn();
}

#[test]
fn testInsertableColumn() {
    mapping_driver_test_case::testInsertableColumn();
}

#[test]
fn testUpdatableColumn() {
    mapping_driver_test_case::testUpdatableColumn();
}

#[test]
fn testEnumType() {
    mapping_driver_test_case::testEnumType();
}

#[test]
fn testCustomNamingStrategyIsRespected() {
    mapping_driver_test_case::testCustomNamingStrategyIsRespected();
}

// AttributeDriverTest

#[test]
fn testDriverCanAcceptClassLocator() {
    let city = model("Cache\\City");
    let class_locator = Arc::new(ClassNames::new([city.clone()]));

    models::register();
    let driver = AttributeDriver::new(
        php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths::Locator(class_locator),
        true,
    )
    .unwrap();

    assert!(
        driver.colocated().get_paths().is_empty(),
        "Directory paths must be empty, since file paths are used"
    );
    assert_eq!(vec![city], driver.get_all_class_names().unwrap());
}

#[test]
fn testOriginallyNestedAttributesDeclaredWithoutOriginalParent() {
    let factory = factory();

    let metadata = factory
        .get_metadata_for(&ns("AttributeEntityWithoutOriginalParents"))
        .unwrap();
    let metadata = metadata.read();

    assert_array(
        php_array! {
            "name" => "AttributeEntityWithoutOriginalParents",
            "uniqueConstraints" => php_array! { "foo" => php_array! { "columns" => php_array!["id"] } },
            "indexes" => php_array! { "bar" => php_array! { "columns" => php_array!["id"] } },
        },
        &metadata.table,
    );
    assert_eq!(
        vec!["assoz_id", "assoz_id"],
        metadata
            .association_mappings
            .get("assoc")
            .unwrap()
            .join_table_columns
    );
}

#[test]
fn testIsTransient() {
    let driver = attribute_driver();

    assert!(driver.is_transient("stdClass").unwrap());

    assert!(driver.is_transient(&ns("AttributeTransientClass")).unwrap());

    assert!(
        !driver
            .is_transient(&ns("AttributeEntityWithoutOriginalParents"))
            .unwrap()
    );

    assert!(
        !driver
            .is_transient(&ns("AttributeEntityStartingWithRepeatableAttributes"))
            .unwrap()
    );
}

fn join_columns(columns: &[JoinColumnMapping]) -> Value {
    let mut list = PhpArray::new();
    for column in columns {
        let _ = list.push(Value::Array(column.to_array()));
    }
    Value::Array(list)
}

#[test]
fn testManyToManyAssociationWithNestedJoinColumns() {
    let factory = factory();

    let metadata = factory
        .get_metadata_for(&ns("Fixtures\\AttributeEntityWithNestedJoinColumns"))
        .unwrap();
    let metadata = metadata.read();
    let join_table = metadata
        .association_mappings
        .get("assoc")
        .unwrap()
        .join_table
        .clone()
        .unwrap();

    let expected = |name: &str| {
        JoinColumnMapping::from_mapping_array(&php_array! {
            "name" => name,
            "referencedColumnName" => name,
            "unique" => false,
            "nullable" => Value::Null,
            "onDelete" => Value::Null,
            "columnDefinition" => Value::Null,
        })
        .unwrap()
    };
    assert!(phpunit_equals(
        &(join_columns(&[expected("assoz_id")])),
        &(join_columns(&join_table.join_columns)),
    ));
    assert!(phpunit_equals(
        &(join_columns(&[expected("inverse_assoz_id")])),
        &(join_columns(&join_table.inverse_join_columns)),
    ));
}

#[test]
fn testCompositeIdPositionOrdering() {
    let factory = factory();

    let metadata = factory
        .get_metadata_for(&ns("Fixtures\\CompositeIdWithPosition"))
        .unwrap();

    assert_eq!(vec!["first", "second"], metadata.read().identifier);
}

#[test]
fn testItThrowsWhenSettingReportFieldsWhereDeclaredToFalse() {
    let error = AttributeDriver::new(Vec::<String>::new(), false).unwrap_err();
    assert_eq!("InvalidArgumentException", error.class);
}

#[test]
fn testMappedSuperclassWithRepository() {
    mapping_driver_test_case::testMappedSuperclassWithRepository();
}
