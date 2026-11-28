//! Port of `BasicInheritanceMappingTest`: the metadata factory on the test
//! entity manager's configuration (the attribute driver, `SQLitePlatform`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/BasicInheritanceMappingTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::vectors::phpunit_equals;
use php_value::{Thrown, Value, php_array};
use php2rust_doctrine_orm::id::IdGenerator;
use php2rust_doctrine_orm::mapping::class_metadata::ClassMetadata;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{
    ClassMetadataFactory, FactoryConfiguration, mapping_exception,
};
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, RuntimeReflectionService,
};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

const NS: &str = "Doctrine\\Tests\\ORM\\Mapping\\";
const DDC869: &str = "Doctrine\\Tests\\Models\\DDC869\\";
const COMPANY_FIX_CONTRACT: &str = "Doctrine\\Tests\\Models\\Company\\CompanyFixContract";

fn ns(class: &str) -> String {
    format!("{NS}{class}")
}

fn set_up() -> ClassMetadataFactory {
    models::register();
    let cmf = ClassMetadataFactory::new();
    cmf.set_entity_manager(FactoryConfiguration::new(Arc::new(
        AttributeDriver::new(Vec::<String>::new(), true).unwrap(),
    )));
    cmf
}

fn text(error: &Thrown) -> String {
    String::from_utf8_lossy(error.message.as_bytes()).into_owned()
}

fn expect_mapping_exception<T>(result: Result<T, Thrown>) -> Thrown {
    let Err(error) = result else {
        panic!("expected a MappingException");
    };
    assert!(
        thrown_is_a(&error, mapping_exception::CLASS),
        "{}: {}",
        error.class,
        text(&error)
    );
    error
}

#[test]
fn testGetMetadataForTransientClassThrowsException() {
    let cmf = set_up();
    expect_mapping_exception(cmf.get_metadata_for(&ns("TransientBaseClass")));
}

#[test]
fn testGetMetadataForSubclassWithTransientBaseClass() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("EntitySubClass")).unwrap();
    let class = class.read();

    assert_eq!(class.sub_classes, [] as [std::string::String; 0]);
    assert_eq!(class.parent_classes, [] as [std::string::String; 0]);
    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("name").is_some());
}

#[test]
fn testGetMetadataForSubclassWithMappedSuperclass() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("EntitySubClass2")).unwrap();
    let class = class.read();

    assert_eq!(class.sub_classes, [] as [std::string::String; 0]);
    assert_eq!(class.parent_classes, [] as [std::string::String; 0]);

    assert!(class.field_mappings.get("mapped1").is_some());
    assert!(class.field_mappings.get("mapped2").is_some());
    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("name").is_some());

    assert_eq!(None, class.field_mappings.get("mapped1").unwrap().inherited);
    assert_eq!(None, class.field_mappings.get("mapped2").unwrap().inherited);
    assert!(class.field_mappings.get("transient").is_none());

    assert!(class.association_mappings.get("mappedRelated1").is_some());
}

#[test]
fn testGetMetadataForSubclassWithMappedSuperclassWithRepository() {
    let cmf = set_up();
    let repository = format!("{DDC869}DDC869PaymentRepository");
    let class = cmf
        .get_metadata_for(&format!("{DDC869}DDC869CreditCardPayment"))
        .unwrap();
    let class = class.read();

    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("value").is_some());
    assert!(class.field_mappings.get("creditCardNumber").is_some());
    assert_eq!(
        Some(&repository),
        class.custom_repository_class_name.as_ref()
    );

    let class = cmf
        .get_metadata_for(&format!("{DDC869}DDC869ChequePayment"))
        .unwrap();
    let class = class.read();

    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("value").is_some());
    assert!(class.field_mappings.get("serialNumber").is_some());
    assert_eq!(
        Some(&repository),
        class.custom_repository_class_name.as_ref()
    );

    // override repositoryClass
    let class = cmf.get_metadata_for(&ns("SubclassWithRepository")).unwrap();
    let class = class.read();

    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("value").is_some());
    assert_eq!(
        Some("Doctrine\\ORM\\EntityRepository"),
        class.custom_repository_class_name.as_deref()
    );
}

#[test]
fn testSerializationWithPrivateFieldsFromMappedSuperclass() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("EntitySubClass2")).unwrap();

    let class2 = ClassMetadata::unserialize(class.serialize().as_bytes()).unwrap();
    class2
        .write()
        .wakeup_reflection(&RuntimeReflectionService::new())
        .unwrap();
    let class2 = class2.read();

    assert!(class2.property_accessors.get("mapped1").is_some());
    assert!(class2.property_accessors.get("mapped2").is_some());
    assert!(class2.property_accessors.get("mappedRelated1").is_some());
}

#[test]
fn testUnmappedSuperclassInHierarchy() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("HierarchyD")).unwrap();
    let class = class.read();

    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("a").is_some());
    assert!(class.field_mappings.get("d").is_some());
}

#[test]
fn testUnmappedEntityInHierarchy() {
    let cmf = set_up();
    let error = expect_mapping_exception(cmf.get_metadata_for(&ns("HierarchyE")));
    assert!(text(&error).contains(
        "Entity 'Doctrine\\Tests\\ORM\\Mapping\\HierarchyBEntity' has to be part of the discriminator map of 'Doctrine\\Tests\\ORM\\Mapping\\HierarchyBase' to be properly mapped in the inheritance hierarchy. Alternatively you can make 'Doctrine\\Tests\\ORM\\Mapping\\HierarchyBEntity' an abstract class to avoid this exception from occurring."
    ), "{}", text(&error));
}

#[test]
fn testMappedSuperclassWithId() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("SuperclassEntity")).unwrap();

    assert!(class.read().field_mappings.get("id").is_some());
}

#[test]
fn testSequenceDefinitionInHierarchyWithSandwichMappedSuperclass() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("HierarchyD")).unwrap();
    let class = class.read();

    assert!(matches!(
        class.id_generator,
        Some(IdGenerator::Sequence { .. })
    ));
    assert!(phpunit_equals(
        &Value::Array(
            php_array! { "allocationSize" => 1, "initialValue" => 10, "sequenceName" => "foo" }
        ),
        &Value::Array(class.sequence_generator_definition.clone().unwrap()),
    ));
}

#[test]
fn testMappedSuperclassIndex() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("EntityIndexSubClass")).unwrap();
    let class = class.read();

    let has_key = |index_type: &str, key: &str| match class.table.get(&index_type.into()) {
        Some(Value::Array(indexes)) => indexes.get(&key.into()).is_some(),
        _ => false,
    };
    assert!(class.field_mappings.get("mapped1").is_some());
    assert!(has_key("uniqueConstraints", "IDX_NAME_INDEX"));
    assert!(has_key("uniqueConstraints", "IDX_MAPPED1_INDEX"));
    assert!(has_key("indexes", "IDX_MAPPED2_INDEX"));
}

/// `invalidHierarchyDeclarationClasses()`.
const INVALID_HIERARCHY_DECLARATION_CLASSES: [(&str, &str, &str); 5] = [
    (
        "concrete Entity root and child class, direct inheritance",
        "InvalidEntityRoot",
        "InvalidEntityRootChild",
    ),
    (
        "concrete Entity root and abstract child class, direct inheritance",
        "InvalidEntityRoot",
        "InvalidEntityRootAbstractChild",
    ),
    (
        "abstract Entity root and concrete child class, direct inheritance",
        "InvalidAbstractEntityRoot",
        "InvalidAbstractEntityRootChild",
    ),
    (
        "abstract Entity root and abstract child class, direct inheritance",
        "InvalidAbstractEntityRoot",
        "InvalidAbstractEntityRootAbstractChild",
    ),
    (
        "complex example (Entity Root -> Mapped Superclass -> transient class -> Entity)",
        "InvalidComplexRoot",
        "InvalidComplexEntity",
    ),
];

#[test]
fn testUndeclaredHierarchyRejection() {
    for (data_set, root_entity, child_class) in INVALID_HIERARCHY_DECLARATION_CLASSES {
        let cmf = set_up();
        let (root_entity, child_class) = (ns(root_entity), ns(child_class));
        let error = expect_mapping_exception(cmf.get_metadata_for(&child_class));
        assert!(
            text(&error).contains(&format!(
                "Entity class '{child_class}' is a subclass of the root entity class '{root_entity}', but no inheritance mapping type was declared."
            )),
            "{data_set}: {}",
            text(&error)
        );
    }
}

#[test]
fn testEmbeddableSubclassDoesNotRequireInheritanceTypeDeclaration() {
    let cmf = set_up();
    let class = cmf.get_metadata_for(&ns("EmbeddableSubclass")).unwrap();

    assert!(class.read().is_embedded_class);
}

#[test]
fn testInvalidOverrideFieldInheritedFromEntity() {
    let cmf = set_up();
    let cm = cmf.get_metadata_for(COMPANY_FIX_CONTRACT).unwrap();

    let error = expect_mapping_exception(
        cm.write()
            .set_attribute_override("completed", &php_array! { "name" => "other_column_name" }),
    );
    assert!(text(&error).contains(
        "Overrides are only allowed for fields or associations declared in mapped superclasses or traits."
    ));
}

#[test]
fn testInvalidOverrideAssociationInheritedFromEntity() {
    let cmf = set_up();
    let cm = cmf.get_metadata_for(COMPANY_FIX_CONTRACT).unwrap();

    let error = expect_mapping_exception(cm.write().set_association_override(
        "salesPerson",
        &php_array! { "inversedBy" => "other_inversed_by_name" },
    ));
    assert_eq!(
        "Overrides are only allowed for fields or associations declared in mapped superclasses or traits. This is not the case for Doctrine\\Tests\\Models\\Company\\CompanyFixContract::salesPerson, which was inherited from Doctrine\\Tests\\Models\\Company\\CompanyContract.",
        text(&error)
    );
}
