//! Port of `XmlMappingDriverTest`, and of the `MappingDriverTestCase` tests
//! it inherits (in `common/mapping_driver_test_case.rs`), over a copy of
//! the mapping files in `tests/data/Mapping`. The data providers' cases
//! run in one test each. XSD validation is on, as upstream's
//! `loadDriver()` has it, but php2rust-dom cannot read
//! `doctrine-mapping.xsd`, so the documents are not validated (the tests
//! that expect a validation error are pending).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/XmlMappingDriverTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
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

use php_value::Value;
use php_value::vectors::phpunit_equals;
use php2rust_doctrine_orm::mapping::class_metadata::sort_direction;
use php2rust_doctrine_orm::mapping::driver::XmlDriver;
use php2rust_doctrine_orm::mapping::driver::xml_driver::DEFAULT_FILE_EXTENSION;
use php2rust_doctrine_orm::mapping::embedded_class_mapping::ColumnPrefix;
use php2rust_doctrine_orm::mapping::{ClassMetadata, ClassMetadataInfo, EmbeddedClassMapping};
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_persistence::mapping::driver::{FileDriver, MappingDriver};
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, RuntimeReflectionService,
};

use mapping_driver_test_case::{
    configuration, create_class_metadata, create_class_metadata_factory,
    create_class_metadata_with, model, ns, text,
};

/// Uses the XML driver (`$this->loadDriver() instanceof XmlDriver`).
pub const USES_XML_DRIVER: bool = true;

/// `__DIR__ . DIRECTORY_SEPARATOR . $directory`.
fn mapping_dir(directory: &str) -> String {
    format!(
        "{}/tests/data/Mapping/{directory}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn xml_driver(directory: &str, is_xsd_validation_enabled: bool) -> FileDriver<XmlDriver> {
    models::register();
    XmlDriver::create(
        mapping_dir(directory).as_str(),
        DEFAULT_FILE_EXTENSION,
        is_xsd_validation_enabled,
    )
}

/// `loadDriver()`.
pub fn load_driver() -> Arc<dyn MappingDriver> {
    Arc::new(xml_driver("xml", true))
}

/// `new ClassMetadata($class)` with `initializeReflection()`, loaded by
/// the driver.
fn load_metadata(driver: &dyn MappingDriver, class: &str) -> ClassMetadataInfo {
    let mut info = ClassMetadataInfo::new(class, None, None);
    info.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    let metadata = ClassMetadata::from_info(info);
    driver.load_metadata_for_class(class, &metadata).unwrap();
    metadata.read().clone()
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

// XmlMappingDriverTest

#[test]
fn testClassTableInheritanceDiscriminatorMap() {
    let class = load_metadata(load_driver().as_ref(), &ns("CTI"));

    let expected_map = php_value::php_array! {
        "foo" => ns("CTIFoo"),
        "bar" => ns("CTIBar"),
        "baz" => ns("CTIBaz"),
    };

    assert_eq!(3, class.discriminator_map.len());
    assert!(phpunit_equals(
        &Value::Array(expected_map),
        &Value::Array(class.discriminator_map.clone())
    ));
}

#[test]
fn testFailingSecondLevelCacheAssociation() {
    let mapping_driver = load_driver();
    let class = ClassMetadata::from_info(ClassMetadataInfo::new(ns("XMLSLC"), None, None));
    let error = mapping_driver
        .load_metadata_for_class(&ns("XMLSLC"), &class)
        .unwrap_err();

    assert!(
        php2rust_doctrine_persistence::php_reflection::thrown_is_a(
            &error,
            "Doctrine\\ORM\\Cache\\CacheException"
        ),
        "{}",
        error.class
    );
    assert_eq!(
        "Entity association field \"Doctrine\\Tests\\ORM\\Mapping\\XMLSLC#foo\" not configured as part of the second-level cache.",
        text(&error)
    );
}

#[test]
fn testIdentifierWithAssociationKey() {
    let factory = create_class_metadata_factory(configuration());

    let class = factory
        .get_metadata_for(&model("DDC117\\DDC117Translation"))
        .unwrap();
    let class = class.read();

    assert_eq!(vec!["language", "article"], class.identifier);
    let article = class.association_mappings.get("article");
    assert!(article.is_some());
    assert_eq!(Some(true), article.unwrap().id);
}

#[test]
fn testCompositeIdPositionOrdering() {
    let class = create_class_metadata(&ns("Fixtures\\CompositeIdWithPosition"));

    assert_eq!(vec!["first", "second"], class.identifier);
}

#[test]
fn testEmbeddableMapping() {
    let class = create_class_metadata(&model("ValueObjects\\Name"));

    assert!(class.is_embedded_class);
}

#[test]
fn testEmbeddableSubclassDoesNotRequireInheritanceTypeDeclaration() {
    let factory = create_class_metadata_factory(configuration());

    let class = factory
        .get_metadata_for(&model("ValueObjects\\EmbeddableSubclass"))
        .unwrap();
    let class = class.read();

    assert!(class.is_embedded_class);
    assert!(class.has_field("value"));
    assert!(class.has_field("extraValue"));
}

#[test]
fn testEmbeddedMappingsWithUseColumnPrefix() {
    let factory = create_class_metadata_factory(configuration());

    let class = factory
        .get_metadata_for(&model("DDC3293\\DDC3293UserPrefixed"))
        .unwrap();

    assert_eq!(
        ColumnPrefix::Prefix("__prefix__".to_owned()),
        class
            .read()
            .embedded_classes
            .get("address")
            .unwrap()
            .column_prefix
    );
}

#[test]
fn testEmbeddedMappingsWithFalseUseColumnPrefix() {
    let factory = create_class_metadata_factory(configuration());

    let class = factory
        .get_metadata_for(&model("DDC3293\\DDC3293User"))
        .unwrap();

    assert_eq!(
        ColumnPrefix::False,
        class
            .read()
            .embedded_classes
            .get("address")
            .unwrap()
            .column_prefix
    );
}

#[test]
fn testEmbeddedMapping() {
    let class = create_class_metadata(&model("ValueObjects\\Person"));

    let expected = EmbeddedClassMapping::from_mapping_array(&php_value::php_array! {
        "class" => model("ValueObjects\\Name"),
        "columnPrefix" => "nm_",
        "declaredField" => Value::Null,
        "originalField" => Value::Null,
    })
    .unwrap();
    assert_eq!(
        vec!["name"],
        class.embedded_classes.keys().collect::<Vec<_>>()
    );
    assert_eq!(&expected, class.embedded_classes.get("name").unwrap());
}

#[test]
fn testItMentionsFilenameAndEntityNameOnInvalidMapping() {
    let error =
        create_class_metadata_with(&model("Generic\\BooleanModel"), None, None).unwrap_err();

    assert!(
        php2rust_doctrine_persistence::php_reflection::thrown_is_a(
            &error,
            "Doctrine\\Persistence\\Mapping\\MappingException"
        ),
        "{}",
        error.class
    );
    assert_eq!(
        "Invalid mapping file 'Doctrine.Tests.Models.Generic.BooleanModel.dcm.xml' for class 'Doctrine\\Tests\\Models\\Generic\\BooleanModel'.",
        text(&error)
    );
}

/// `dataValidSchema()`.
#[test]
fn testValidateXmlSchema() {
    let cases: [(String, &str, Vec<&str>, Vec<&str>); 3] = [
        (
            ns("User"),
            "cms_users",
            vec!["name", "email", "version", "indexed", "id"],
            vec!["address", "phonenumbers", "groups"],
        ),
        (
            model("DDC889\\DDC889Entity"),
            "DDC889Entity",
            vec![],
            vec![],
        ),
        (
            model("DDC889\\DDC889SuperClass"),
            "DDC889SuperClass",
            vec!["name"],
            vec![],
        ),
    ];
    for (class, table_name, field_names, association_names) in cases {
        let metadata = create_class_metadata(&class);

        assert_eq!(table_name, metadata.get_table_name().unwrap());
        assert_eq!(field_names, metadata.get_field_names());
        assert_eq!(association_names, metadata.get_association_names());
    }
}

#[test]
fn testOneToManyDefaultOrderByAsc() {
    let class = load_metadata(load_driver().as_ref(), &model("GH7141\\GH7141Article"));

    assert!(phpunit_equals(
        &sort_direction(SortDirection::Ascending),
        class
            .association_mappings
            .get("tags")
            .unwrap()
            .order_by
            .get(&"position".into())
            .unwrap()
    ));
}

#[test]
fn testManyToManyDefaultOrderByAsc() {
    let class = load_metadata(load_driver().as_ref(), &model("GH7316\\GH7316Article"));

    assert!(phpunit_equals(
        &sort_direction(SortDirection::Ascending),
        class
            .association_mappings
            .get("tags")
            .unwrap()
            .order_by
            .get(&"position".into())
            .unwrap()
    ));
}

#[test]
fn testClassNameInFieldOrId() {
    let class = load_metadata(load_driver().as_ref(), &model("Project\\Project"));

    let id = class.get_field_mapping("id").unwrap();
    let name = class.get_field_mapping("name").unwrap();

    assert_eq!(model("Project\\ProjectId"), id.r#type);
    assert_eq!(model("Project\\ProjectName"), name.r#type);
}

#[test]
fn testDisablingXmlValidationIsPossible() {
    xml_driver("xml", false);
}

#[test]
fn testXmlValidationDisabled() {
    let driver = xml_driver("invalid_xml", false);

    let class = load_metadata(&driver, &model("InvalidXml"));

    assert_eq!(1, class.field_mappings.len());
}

#[test]
fn testXmlCustomForeignKeyNames() {
    let class = create_class_metadata(&ns("XmlCustomFKArticle"));

    // Test ManyToOne with custom FK name
    let author_mapping = class.get_association_mapping("author").unwrap();
    assert_eq!(1, author_mapping.join_columns.len());
    assert_eq!(
        Some("fk_xml_article_author"),
        author_mapping.join_columns[0].foreign_key_name.as_deref()
    );

    // Test ManyToMany with custom FK names
    let tags_mapping = class.get_association_mapping("tags").unwrap();
    let join_table = tags_mapping.join_table.as_ref();
    assert!(join_table.is_some());
    assert_eq!(
        Some("fk_xml_article_tag_article"),
        join_table.unwrap().foreign_key_name.as_deref()
    );
    assert_eq!(
        Some("fk_xml_article_tag_tag"),
        join_table.unwrap().inverse_foreign_key_name.as_deref()
    );
}
