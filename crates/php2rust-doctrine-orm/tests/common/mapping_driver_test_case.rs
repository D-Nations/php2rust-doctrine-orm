//! `MappingDriverTestCase`: the tests every mapping driver's test runs,
//! over the `load_driver()` of the test binary (`crate::load_driver`).
//! Each test binary's `#[test]` calls them; `#[Depends]` tests rebuild the
//! metadata they depend on; the entity manager is the factory's
//! `FactoryConfiguration`; PHPUnit's `assertEquals` on arrays is
//! `phpunit_equals`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/MappingDriverTestCase.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::vectors::phpunit_equals;
use php_value::{PhpArray, Thrown, Value, php_array};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::mapping::class_metadata::{
    self as cm, ClassMetadataInfo, sort_direction,
};
use php2rust_doctrine_orm::mapping::naming_strategy::{CASE_UPPER, UnderscoreNamingStrategy};
use php2rust_doctrine_orm::mapping::typed_field_mapper::DefaultTypedFieldMapper;
use php2rust_doctrine_orm::mapping::{
    ClassMetadata, ClassMetadataFactory, DiscriminatorColumnMapping, FactoryConfiguration,
    NamingStrategy, mapping_exception,
};
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, RuntimeReflectionService,
};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

pub const NS: &str = "Doctrine\\Tests\\ORM\\Mapping\\";
pub const MODELS: &str = "Doctrine\\Tests\\Models\\";
/// `https://github.com/doctrine/orm/issues/11313`.
pub const STRING_ORDER_BY_DEPRECATION: &str = "https://github.com/doctrine/orm/issues/11313";

pub fn ns(class: &str) -> String {
    format!("{NS}{class}")
}

pub fn model(class: &str) -> String {
    format!("{MODELS}{class}")
}

pub fn text(error: &Thrown) -> String {
    String::from_utf8_lossy(error.message.as_bytes()).into_owned()
}

pub fn expect_mapping_exception<T>(result: Result<T, Thrown>) -> Thrown {
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

pub fn assert_equals(expected: &Value, actual: &Value) {
    assert!(
        phpunit_equals(expected, actual),
        "{expected:?} != {actual:?}"
    );
}

/// `loadDriver()`: the driver of the test binary.
pub fn load_driver() -> Arc<dyn MappingDriver> {
    crate::models::register();
    crate::load_driver()
}

/// `createClassMetadata()`.
pub fn create_class_metadata_with(
    entity_class_name: &str,
    naming_strategy: Option<Arc<dyn NamingStrategy>>,
    typed_field_mapper: Option<
        Arc<dyn php2rust_doctrine_orm::mapping::typed_field_mapper::TypedFieldMapper>,
    >,
) -> Result<ClassMetadataInfo, Thrown> {
    let mapping_driver = load_driver();

    let mut class = ClassMetadataInfo::new(entity_class_name, naming_strategy, typed_field_mapper);
    class.initialize_reflection(&RuntimeReflectionService::new())?;
    let class = ClassMetadata::from_info(class);
    mapping_driver.load_metadata_for_class(entity_class_name, &class)?;

    Ok(class.read().clone())
}

pub fn create_class_metadata(entity_class_name: &str) -> ClassMetadataInfo {
    create_class_metadata_with(entity_class_name, None, None).unwrap()
}

/// `createClassMetadataFactory()`, on the test entity manager's
/// configuration.
pub fn configuration() -> FactoryConfiguration {
    FactoryConfiguration::new(load_driver())
}

pub fn create_class_metadata_factory(configuration: FactoryConfiguration) -> ClassMetadataFactory {
    let factory = ClassMetadataFactory::new();
    factory.set_entity_manager(configuration);
    factory
}

pub fn factory() -> ClassMetadataFactory {
    create_class_metadata_factory(configuration())
}

pub fn table(class: &ClassMetadataInfo, key: &str) -> Value {
    class.table.get(&key.into()).cloned().unwrap_or(Value::Null)
}

/// `testEntityTableNameAndInheritance()`, which the `#[Depends]` tests
/// start from.
pub fn user() -> ClassMetadataInfo {
    create_class_metadata(&ns("User"))
}

pub fn testEntityTableNameAndInheritance() {
    let class = user();

    assert_eq!("cms_users", class.get_table_name().unwrap());
    assert_eq!(cm::INHERITANCE_TYPE_NONE, class.inheritance_type);
}

pub fn testEntityIndexes() {
    let class = user();
    assert!(
        class.table.get(&"indexes".into()).is_some(),
        "ClassMetadata should have indexes key in table property."
    );
    assert_equals(
        &Value::Array(php_array! {
            "name_idx" => php_array! { "columns" => php_array!["name"] },
            0 => php_array! { "columns" => php_array!["user_email"] },
            "fields" => php_array! { "fields" => php_array!["name", "email"] },
        }),
        &table(&class, "indexes"),
    );
}

pub fn testEntityIncorrectIndexes() {
    expect_mapping_exception(create_class_metadata_with(
        &ns("UserIncorrectIndex"),
        None,
        None,
    ));
}

pub fn testEntityIndexFlagsAndPartialIndexes() {
    let class = create_class_metadata(&ns("Comment"));

    assert_equals(
        &Value::Array(php_array! {
            0 => php_array! {
                "columns" => php_array!["content"],
                "flags" => php_array!["fulltext"],
                "options" => php_array! { "where" => "content IS NOT NULL" },
            },
        }),
        &table(&class, "indexes"),
    );
}

pub fn testEntityUniqueConstraints() {
    let class = user();
    assert!(
        class.table.get(&"uniqueConstraints".into()).is_some(),
        "ClassMetadata should have uniqueConstraints key in table property when Unique Constraints are set."
    );

    assert_equals(
        &Value::Array(php_array! {
            "search_idx" => php_array! {
                "columns" => php_array!["name", "user_email"],
                "options" => php_array! { "where" => "name IS NOT NULL" },
            },
            "phone_idx" => php_array! { "fields" => php_array!["name", "phone"] },
        }),
        &table(&class, "uniqueConstraints"),
    );
}

pub fn testEntityIncorrectUniqueContraint() {
    expect_mapping_exception(create_class_metadata_with(
        &ns("UserIncorrectUniqueConstraint"),
        None,
        None,
    ));
}

pub fn testEntityOptions() {
    let class = user();
    assert!(
        class.table.get(&"options".into()).is_some(),
        "ClassMetadata should have options key in table property."
    );

    assert_equals(
        &Value::Array(php_array! {
            "foo" => "bar",
            "baz" => php_array! { "key" => "val" },
        }),
        &table(&class, "options"),
    );
}

pub fn testEntitySequence() {
    let class = user();
    let definition = class
        .sequence_generator_definition
        .clone()
        .expect("No Sequence Definition set on this driver.");
    assert_equals(
        &Value::Array(php_array! {
            "sequenceName" => "tablename_seq",
            "allocationSize" => 100,
            "initialValue" => 1,
        }),
        &Value::Array(definition),
    );
}

pub fn testEntityCustomGenerator() {
    let class = create_class_metadata(&ns("Animal"));

    assert_eq!(
        cm::GENERATOR_TYPE_CUSTOM,
        class.generator_type,
        "Generator Type"
    );
    assert_equals(
        &Value::Array(php_array! { "class" => "stdClass" }),
        &Value::Array(class.custom_generator_definition.clone().unwrap()),
    );
}

pub fn testFieldMappings() {
    let class = user();
    assert_eq!(5, class.field_mappings.len());
    assert!(class.field_mappings.get("id").is_some());
    assert!(class.field_mappings.get("name").is_some());
    assert!(class.field_mappings.get("email").is_some());
    assert!(class.field_mappings.get("version").is_some());
    assert!(class.field_mappings.get("indexed").is_some());
}

pub fn testVersionedField() {
    let class = user();
    assert!(class.is_versioned);
    assert_eq!(Some("version"), class.version_field.as_deref());

    assert_eq!(None, class.field_mappings.get("version").unwrap().version);
}

pub fn testFieldMappingsColumnNames() {
    let class = user();
    assert_eq!("id", class.field_mappings.get("id").unwrap().column_name);
    assert_eq!(
        "name",
        class.field_mappings.get("name").unwrap().column_name
    );
    assert_eq!(
        "user_email",
        class.field_mappings.get("email").unwrap().column_name
    );
}

pub fn testStringFieldMappings() {
    let class = user();
    let name = class.field_mappings.get("name").unwrap();
    assert_eq!("string", name.r#type);
    assert_eq!(Some(50), name.length);
    assert_eq!(Some(true), name.nullable);
    assert_eq!(Some(true), name.unique);
    assert_eq!(
        Some(true),
        class.field_mappings.get("indexed").unwrap().index
    );
}

pub fn testFieldTypeFromReflection() {
    let class = create_class_metadata(&model("TypedProperties\\UserTyped"));

    assert_eq!(Some("integer"), class.get_type_of_field("id").as_deref());
    assert_eq!(
        Some("string"),
        class.get_type_of_field("username").as_deref()
    );
    assert_eq!(
        Some("dateinterval"),
        class.get_type_of_field("dateInterval").as_deref()
    );
    assert_eq!(
        Some("datetime"),
        class.get_type_of_field("dateTime").as_deref()
    );
    assert_eq!(
        Some("datetime_immutable"),
        class.get_type_of_field("dateTimeImmutable").as_deref()
    );
    assert_eq!(Some("json"), class.get_type_of_field("array").as_deref());
    assert_eq!(
        Some("boolean"),
        class.get_type_of_field("boolean").as_deref()
    );
    assert_eq!(Some("float"), class.get_type_of_field("float").as_deref());

    assert_eq!(
        model("CMS\\CmsEmail"),
        class
            .get_association_mapping("email")
            .unwrap()
            .target_entity
    );
    assert_eq!(
        model("CMS\\CmsEmail"),
        class
            .get_association_mapping("mainEmail")
            .unwrap()
            .target_entity
    );
    assert_eq!(
        model("TypedProperties\\Contact"),
        class.embedded_classes.get("contact").unwrap().class
    );
}

pub fn testCustomFieldTypeFromReflection() {
    const CUSTOM_ID_OBJECT: &str = "Doctrine\\Tests\\DbalTypes\\CustomIdObject";
    const CUSTOM_ID_OBJECT_TYPE: &str = "Doctrine\\Tests\\DbalTypes\\CustomIdObjectType";
    const CUSTOM_INT_TYPE: &str = "Doctrine\\Tests\\DbalTypes\\CustomIntType";
    let class = create_class_metadata_with(
        &model("TypedProperties\\UserTypedWithCustomTypedField"),
        None,
        Some(Arc::new(DefaultTypedFieldMapper::new(vec![
            (
                CUSTOM_ID_OBJECT.to_owned(),
                CUSTOM_ID_OBJECT_TYPE.to_owned(),
            ),
            ("int".to_owned(), CUSTOM_INT_TYPE.to_owned()),
        ]))),
    )
    .unwrap();

    assert_eq!(
        Some(CUSTOM_ID_OBJECT_TYPE),
        class.get_type_of_field("customId").as_deref()
    );
    assert_eq!(
        Some(CUSTOM_INT_TYPE),
        class.get_type_of_field("customIntTypedField").as_deref()
    );
}

pub fn options(class: &ClassMetadataInfo, field: &str) -> Value {
    Value::Array(
        class
            .field_mappings
            .get(field)
            .unwrap()
            .options
            .clone()
            .unwrap_or_default(),
    )
}

pub fn testFieldOptions() {
    let class = user();
    let expected =
        php_array! { "foo" => "bar", "baz" => php_array! { "key" => "val" }, "fixed" => false };
    assert_equals(&Value::Array(expected), &options(&class, "name"));
}

pub fn testIdFieldOptions() {
    let class = user();
    assert_equals(
        &Value::Array(php_array! { "foo" => "bar", "unsigned" => false }),
        &options(&class, "id"),
    );
}

pub fn testIdentifier() {
    let class = user();
    assert_eq!(vec!["id"], class.identifier);
    assert_eq!("integer", class.field_mappings.get("id").unwrap().r#type);
    assert_eq!(
        cm::GENERATOR_TYPE_AUTO,
        class.generator_type,
        "ID-Generator is not ClassMetadata::GENERATOR_TYPE_AUTO"
    );
}

pub fn testBooleanValuesForOptionIsSetCorrectly() {
    let class = create_class_metadata(&ns("User"));

    let option = |field: &str, key: &str| match options(&class, field) {
        Value::Array(options) => options.get(&key.into()).cloned(),
        _ => None,
    };
    assert_eq!(Some(Value::Bool(false)), option("id", "unsigned"));
    assert_eq!(Some(Value::Bool(false)), option("name", "fixed"));
}

pub fn testAssociations() {
    let class = user();
    assert_eq!(3, class.association_mappings.len());
}

pub fn testOwningOneToOneAssociation() {
    let class = user();
    let address = class.association_mappings.get("address").unwrap();
    assert!(address.is_owning_side());
    assert_eq!(Some("user"), address.inversed_by.as_deref());
    // Check cascading
    assert!(address.is_cascade_remove());
    assert!(!address.is_cascade_persist());
    assert!(!address.is_cascade_refresh());
    assert!(!address.is_cascade_detach());
}

pub fn testInverseOneToManyAssociation() {
    let class = user();
    let phonenumbers = class.association_mappings.get("phonenumbers").unwrap();
    assert!(!phonenumbers.is_owning_side());
    assert!(phonenumbers.is_cascade_persist());
    assert!(phonenumbers.is_cascade_remove());
    assert!(!phonenumbers.is_cascade_refresh());
    assert!(!phonenumbers.is_cascade_detach());
    assert!(phonenumbers.orphan_removal);

    // Test Order By
    assert_equals(
        &Value::Array(php_array! { "number" => sort_direction(SortDirection::Ascending) }),
        &Value::Array(phonenumbers.order_by.clone()),
    );
}

pub fn testOrderByWithStringValues() {
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(STRING_ORDER_BY_DEPRECATION);

    let class = create_class_metadata(&ns("UserWithStringOrderBy"));

    if crate::USES_XML_DRIVER {
        assert_eq!(0, deprecations::count(STRING_ORDER_BY_DEPRECATION) - before);
    } else {
        // there are 3 associations using the string order value;
        // VerifyDeprecations checks the count is above the expectation
        // (one deprecation per string direction: four here).
        assert!(deprecations::count(STRING_ORDER_BY_DEPRECATION) - before > 3);
    }
    let order_by = |field: &str| {
        Value::Array(
            class
                .association_mappings
                .get(field)
                .unwrap()
                .order_by
                .clone(),
        )
    };
    assert_equals(
        &Value::Array(php_array! { "number" => sort_direction(SortDirection::Ascending) }),
        &order_by("phonenumbersAsc"),
    );
    assert_equals(
        &Value::Array(php_array! { "number" => sort_direction(SortDirection::Descending) }),
        &order_by("phonenumbersDesc"),
    );
    assert_equals(
        &Value::Array(php_array! {
            "name" => sort_direction(SortDirection::Ascending),
            "id" => sort_direction(SortDirection::Descending),
        }),
        &order_by("groupsMixed"),
    );
}

pub fn testOrderByWithInvalidDirection() {
    let error = expect_mapping_exception(create_class_metadata_with(
        &ns("UserWithInvalidOrderBy"),
        None,
        None,
    ));
    assert!(text(&error).contains("INVALID"), "{}", text(&error));
}

pub fn testManyToManyAssociationWithCascadeAll() {
    let class = user();
    let groups = class.association_mappings.get("groups").unwrap();
    assert!(groups.is_owning_side());
    // Make sure that cascade-all works as expected
    assert!(groups.is_cascade_remove());
    assert!(groups.is_cascade_persist());
    assert!(groups.is_cascade_refresh());
    assert!(groups.is_cascade_detach());

    assert!(!groups.is_ordered());
}

pub fn testLifecycleCallbacks() {
    let class = user();
    assert_eq!(2, class.lifecycle_callbacks.len());
    assert_eq!(
        "doStuffOnPrePersist",
        class.lifecycle_callbacks.get("prePersist").unwrap()[0]
    );
    assert_eq!(
        "doStuffOnPostPersist",
        class.lifecycle_callbacks.get("postPersist").unwrap()[0]
    );
}

pub fn testLifecycleCallbacksSupportMultipleMethodNames() {
    let class = user();
    let pre_persist = class.lifecycle_callbacks.get("prePersist").unwrap();
    assert_eq!(2, pre_persist.len());
    assert_eq!("doOtherStuffOnPrePersistToo", pre_persist[1]);
}

pub fn testJoinColumnUniqueAndNullable() {
    let class = user();
    // Non-Nullability of Join Column
    let groups = class.association_mappings.get("groups").unwrap();
    let join_column = &groups.join_table.as_ref().unwrap().join_columns[0];
    assert_eq!(Some(false), join_column.nullable);
    assert_eq!(Some(false), join_column.unique);
}

pub fn testColumnDefinition() {
    let class = user();
    assert_eq!(
        Some("CHAR(32) NOT NULL"),
        class
            .field_mappings
            .get("email")
            .unwrap()
            .column_definition
            .as_deref()
    );
    let groups = class.association_mappings.get("groups").unwrap();
    assert_eq!(
        Some("INT NULL"),
        groups.join_table.as_ref().unwrap().inverse_join_columns[0]
            .column_definition
            .as_deref()
    );
}

pub fn testJoinColumnOnDelete() {
    let class = user();
    assert_eq!(
        Some("CASCADE"),
        class
            .association_mappings
            .get("address")
            .unwrap()
            .join_columns[0]
            .on_delete
            .as_deref()
    );
}

pub fn testDiscriminatorColumnDefaults() {
    let class = create_class_metadata(&ns("Animal"));

    assert_eq!(
        Some(
            DiscriminatorColumnMapping::from_mapping_array(&php_array! {
                "name" => "discr",
                "type" => "string",
                "length" => 32,
                "fieldName" => "discr",
                "columnDefinition" => Value::Null,
                "enumType" => Value::Null,
            })
            .unwrap()
        ),
        class.discriminator_column
    );
}

pub fn testDefaultFieldType() {
    let factory = factory();
    let class = factory
        .get_metadata_for(&model("DDC1476\\DDC1476EntityWithDefaultFieldType"))
        .unwrap();
    let class = class.read();

    let id = class.field_mappings.get("id").unwrap();
    let name = class.field_mappings.get("name").unwrap();

    assert_eq!("string", id.r#type);
    assert_eq!("string", name.r#type);

    assert_eq!("id", id.field_name);
    assert_eq!("name", name.field_name);

    assert_eq!("id", id.column_name);
    assert_eq!("name", name.column_name);

    assert_eq!(cm::GENERATOR_TYPE_NONE, class.generator_type);
}

pub fn testIdentifierColumnDefinition() {
    let class = create_class_metadata(&ns("DDC1170Entity"));

    let definition = |field: &str| {
        class
            .field_mappings
            .get(field)
            .unwrap()
            .column_definition
            .clone()
            .unwrap_or_default()
            .to_lowercase()
    };
    assert_eq!("int unsigned not null", definition("id"));
    assert_eq!("varchar(255) not null", definition("value"));
}

pub fn testNamingStrategy() {
    let mut configuration = configuration();
    configuration.naming_strategy = Arc::new(UnderscoreNamingStrategy::new(CASE_UPPER));
    let factory = create_class_metadata_factory(configuration);

    let class = factory
        .get_metadata_for(&model("DDC1476\\DDC1476EntityWithDefaultFieldType"))
        .unwrap();
    let class = class.read();

    assert_eq!("ID", class.get_column_name("id"));
    assert_eq!("NAME", class.get_column_name("name"));
    assert_eq!(
        Value::from("DDC1476_ENTITY_WITH_DEFAULT_FIELD_TYPE"),
        table(&class, "name")
    );
}

pub fn testDiscriminatorColumnDefinition() {
    let class = create_class_metadata(&ns("DDC807Entity"));

    let column = class.discriminator_column.as_ref().unwrap();
    assert_eq!(
        Some("ENUM('ONE','TWO')"),
        column.column_definition.as_deref()
    );
    assert_eq!("dtype", column.name);
}

pub fn testDiscriminatorColumnEnumTypeDefinition() {
    let class = create_class_metadata(&ns("GH10288EnumTypePerson"));

    let column = class.discriminator_column.as_ref().unwrap();
    assert_eq!(Some(model("GH10288\\GH10288People")), column.enum_type);
    assert_eq!("discr", column.name);
}

pub fn testInvalidEntityOrMappedSuperClassShouldMentionParentClasses() {
    let error = expect_mapping_exception(create_class_metadata_with(
        &model("DDC889\\DDC889Class"),
        None,
        None,
    ));
    assert_eq!(
        "Class \"Doctrine\\Tests\\Models\\DDC889\\DDC889Class\" sub class of \"Doctrine\\Tests\\Models\\DDC889\\DDC889SuperClass\" is not a valid entity or mapped super class.",
        text(&error)
    );
}

pub fn testIdentifierRequiredShouldMentionParentClasses() {
    let factory = factory();

    let error = expect_mapping_exception(factory.get_metadata_for(&model("DDC889\\DDC889Entity")));
    assert_eq!(
        "No identifier/primary key specified for Entity \"Doctrine\\Tests\\Models\\DDC889\\DDC889Entity\" sub class of \"Doctrine\\Tests\\Models\\DDC889\\DDC889SuperClass\". Every Entity must have an identifier/primary key.",
        text(&error)
    );
}

pub fn testInversedByOverrideMapping() {
    let factory = factory();
    let admin_metadata = factory
        .get_metadata_for(&model("DDC3579\\DDC3579Admin"))
        .unwrap();
    let admin_metadata = admin_metadata.read();

    // assert groups association mappings
    let admin_groups = admin_metadata.association_mappings.get("groups").unwrap();

    // assert override
    assert_eq!(Some("admins"), admin_groups.inversed_by.as_deref());
}

pub fn testFetchOverrideMapping() {
    // check override metadata
    let contract_metadata = factory()
        .get_metadata_for(&model("DDC5934\\DDC5934Contract"))
        .unwrap();
    let contract_metadata = contract_metadata.read();

    let members = contract_metadata
        .association_mappings
        .get("members")
        .unwrap();
    assert_eq!(Some(cm::FETCH_EXTRA_LAZY), members.fetch);
}

pub fn assert_array(expected: PhpArray, actual: &PhpArray) {
    assert_equals(&Value::Array(expected), &Value::Array(actual.clone()));
}

#[allow(
    clippy::too_many_lines,
    reason = "the upstream test, assertion by assertion"
)]
pub fn testAssociationOverridesMapping() {
    let factory = factory();
    let admin_metadata = factory
        .get_metadata_for(&model("DDC964\\DDC964Admin"))
        .unwrap();
    let guest_metadata = factory
        .get_metadata_for(&model("DDC964\\DDC964Guest"))
        .unwrap();
    let admin_metadata = admin_metadata.read();
    let guest_metadata = guest_metadata.read();

    // assert groups association mappings
    let guest_groups = guest_metadata.association_mappings.get("groups").unwrap();
    let admin_groups = admin_metadata.association_mappings.get("groups").unwrap();

    // assert not override attributes
    assert_eq!(guest_groups.field_name, admin_groups.field_name);
    assert_eq!(
        guest_groups.r#type().unwrap(),
        admin_groups.r#type().unwrap()
    );
    assert_eq!(guest_groups.inversed_by, admin_groups.inversed_by);
    assert_eq!(guest_groups.is_owning_side(), admin_groups.is_owning_side());
    assert_eq!(guest_groups.fetch, admin_groups.fetch);
    assert_eq!(
        guest_groups.is_cascade_remove(),
        admin_groups.is_cascade_remove()
    );
    assert_eq!(
        guest_groups.is_cascade_persist(),
        admin_groups.is_cascade_persist()
    );
    assert_eq!(
        guest_groups.is_cascade_refresh(),
        admin_groups.is_cascade_refresh()
    );
    assert_eq!(
        guest_groups.is_cascade_detach(),
        admin_groups.is_cascade_detach()
    );

    // assert not override attributes
    let guest_join_table = guest_groups.join_table.as_ref().unwrap();
    assert_eq!("ddc964_users_groups", guest_join_table.name);
    assert_eq!("user_id", guest_join_table.join_columns[0].name);
    assert_eq!("group_id", guest_join_table.inverse_join_columns[0].name);

    assert_array(
        php_array! { "user_id" => "id" },
        &guest_groups.relation_to_source_key_columns,
    );
    assert_array(
        php_array! { "group_id" => "id" },
        &guest_groups.relation_to_target_key_columns,
    );
    assert_eq!(vec!["user_id", "group_id"], guest_groups.join_table_columns);

    let admin_join_table = admin_groups.join_table.as_ref().unwrap();
    assert_eq!("ddc964_users_admingroups", admin_join_table.name);
    assert_eq!("adminuser_id", admin_join_table.join_columns[0].name);
    assert_eq!(
        "admingroup_id",
        admin_join_table.inverse_join_columns[0].name
    );

    assert_array(
        php_array! { "adminuser_id" => "id" },
        &admin_groups.relation_to_source_key_columns,
    );
    assert_array(
        php_array! { "admingroup_id" => "id" },
        &admin_groups.relation_to_target_key_columns,
    );
    assert_eq!(
        vec!["adminuser_id", "admingroup_id"],
        admin_groups.join_table_columns
    );

    // assert address association mappings
    let guest_address = guest_metadata.association_mappings.get("address").unwrap();
    let admin_address = admin_metadata.association_mappings.get("address").unwrap();

    // assert not override attributes
    assert_eq!(guest_address.field_name, admin_address.field_name);
    assert_eq!(
        guest_address.r#type().unwrap(),
        admin_address.r#type().unwrap()
    );
    assert_eq!(guest_address.inversed_by, admin_address.inversed_by);
    assert_eq!(
        guest_address.is_owning_side(),
        admin_address.is_owning_side()
    );
    assert_eq!(guest_address.fetch, admin_address.fetch);
    assert_eq!(
        guest_address.is_cascade_remove(),
        admin_address.is_cascade_remove()
    );
    assert_eq!(
        guest_address.is_cascade_persist(),
        admin_address.is_cascade_persist()
    );
    assert_eq!(
        guest_address.is_cascade_refresh(),
        admin_address.is_cascade_refresh()
    );
    assert_eq!(
        guest_address.is_cascade_detach(),
        admin_address.is_cascade_detach()
    );

    // assert override
    assert_eq!("address_id", guest_address.join_columns[0].name);
    assert_array(
        php_array! { "address_id" => "id" },
        &guest_address.source_to_target_key_columns,
    );
    assert_array(
        php_array! { "address_id" => "address_id" },
        &guest_address.join_column_field_names,
    );
    assert_array(
        php_array! { "id" => "address_id" },
        &guest_address.target_to_source_key_columns,
    );

    assert_eq!("adminaddress_id", admin_address.join_columns[0].name);
    assert_array(
        php_array! { "adminaddress_id" => "id" },
        &admin_address.source_to_target_key_columns,
    );
    assert_array(
        php_array! { "adminaddress_id" => "adminaddress_id" },
        &admin_address.join_column_field_names,
    );
    assert_array(
        php_array! { "id" => "adminaddress_id" },
        &admin_address.target_to_source_key_columns,
    );
}

/// The entries of a name map, sorted: `assertEquals()` on arrays does not
/// compare the order of their keys.
pub fn names(
    map: &php2rust_doctrine_orm::mapping::ordered_map::OrderedMap<String>,
) -> Vec<(&str, &str)> {
    let mut names: Vec<(&str, &str)> = map
        .iter()
        .map(|(key, value)| (key, value.as_str()))
        .collect();
    names.sort_unstable();
    names
}

pub fn testAttributeOverridesMapping() {
    let factory = factory();
    let guest_metadata = factory
        .get_metadata_for(&model("DDC964\\DDC964Guest"))
        .unwrap();
    let admin_metadata = factory
        .get_metadata_for(&model("DDC964\\DDC964Admin"))
        .unwrap();
    let guest_metadata = guest_metadata.read();
    let admin_metadata = admin_metadata.read();

    let id = admin_metadata.field_mappings.get("id").unwrap();
    assert_eq!(Some(true), id.id);
    assert_eq!("id", id.field_name);
    assert_eq!("user_id", id.column_name);
    assert_eq!(
        vec![("user_id", "id"), ("user_name", "name")],
        names(&admin_metadata.field_names)
    );
    assert_eq!(
        vec![("id", "user_id"), ("name", "user_name")],
        names(&admin_metadata.column_names)
    );
    assert_eq!(Some(150), id.length);

    let name = admin_metadata.field_mappings.get("name").unwrap();
    assert_eq!("name", name.field_name);
    assert_eq!("user_name", name.column_name);
    assert_eq!(Some(250), name.length);
    assert_eq!(Some(true), name.nullable);
    assert_ne!(Some(true), name.unique);

    let id = guest_metadata.field_mappings.get("id").unwrap();
    assert_eq!(Some(true), id.id);
    assert_eq!("guest_id", id.column_name);
    assert_eq!("id", id.field_name);
    assert_eq!(
        vec![("guest_id", "id"), ("guest_name", "name")],
        names(&guest_metadata.field_names)
    );
    assert_eq!(
        vec![("id", "guest_id"), ("name", "guest_name")],
        names(&guest_metadata.column_names)
    );
    assert_eq!(Some(140), id.length);

    let name = guest_metadata.field_mappings.get("name").unwrap();
    assert_eq!("name", name.field_name);
    assert_eq!("guest_name", name.column_name);
    assert_eq!(Some(240), name.length);
    assert_ne!(Some(true), name.nullable);
    assert_eq!(Some(true), name.unique);
}

pub fn listener(class: &ClassMetadataInfo, event: &str, index: usize) -> (String, String) {
    let listener = &class.entity_listeners.get(event).unwrap()[index];
    (listener.class.clone(), listener.method.clone())
}

pub fn testEntityListeners() {
    let factory = factory();
    let super_class = factory
        .get_metadata_for(&model("Company\\CompanyContract"))
        .unwrap();
    let flex_class = factory
        .get_metadata_for(&model("Company\\CompanyFixContract"))
        .unwrap();
    let fix_class = factory
        .get_metadata_for(&model("Company\\CompanyFlexContract"))
        .unwrap();
    let _ultra_class = factory
        .get_metadata_for(&model("Company\\CompanyFlexUltraContract"))
        .unwrap();
    let super_class = super_class.read();

    assert_eq!(
        1,
        super_class
            .entity_listeners
            .get(events::PRE_PERSIST)
            .unwrap()
            .len()
    );
    assert_eq!(
        1,
        super_class
            .entity_listeners
            .get(events::POST_PERSIST)
            .unwrap()
            .len()
    );

    let listener_class = model("Company\\CompanyContractListener");
    assert_eq!(
        (listener_class.clone(), "postPersistHandler".to_owned()),
        listener(&super_class, events::POST_PERSIST, 0)
    );
    assert_eq!(
        (listener_class, "prePersistHandler".to_owned()),
        listener(&super_class, events::PRE_PERSIST, 0)
    );

    //Inherited listeners
    assert_eq!(
        fix_class.read().entity_listeners,
        super_class.entity_listeners
    );
    assert_eq!(
        flex_class.read().entity_listeners,
        super_class.entity_listeners
    );
}

pub fn testEntityListenersOverride() {
    let factory = factory();
    let ultra_class = factory
        .get_metadata_for(&model("Company\\CompanyFlexUltraContract"))
        .unwrap();
    let ultra_class = ultra_class.read();

    //overridden listeners
    assert_eq!(
        1,
        ultra_class
            .entity_listeners
            .get(events::POST_PERSIST)
            .unwrap()
            .len()
    );
    assert_eq!(
        3,
        ultra_class
            .entity_listeners
            .get(events::PRE_PERSIST)
            .unwrap()
            .len()
    );

    let contract_listener = model("Company\\CompanyContractListener");
    let ultra_listener = model("Company\\CompanyFlexUltraContractListener");
    assert_eq!(
        (contract_listener.clone(), "postPersistHandler".to_owned()),
        listener(&ultra_class, events::POST_PERSIST, 0)
    );
    assert_eq!(
        (contract_listener, "prePersistHandler".to_owned()),
        listener(&ultra_class, events::PRE_PERSIST, 0)
    );
    assert_eq!(
        (ultra_listener.clone(), "prePersistHandler1".to_owned()),
        listener(&ultra_class, events::PRE_PERSIST, 1)
    );
    assert_eq!(
        (ultra_listener, "prePersistHandler2".to_owned()),
        listener(&ultra_class, events::PRE_PERSIST, 2)
    );
}

pub fn testEntityListenersNamingConvention() {
    let factory = factory();
    let metadata = factory.get_metadata_for(&model("CMS\\CmsAddress")).unwrap();
    let metadata = metadata.read();

    let listener_class = model("CMS\\CmsAddressListener");
    for event in [
        events::POST_PERSIST,
        events::PRE_PERSIST,
        events::POST_UPDATE,
        events::PRE_UPDATE,
        events::POST_REMOVE,
        events::PRE_REMOVE,
        events::POST_LOAD,
        events::PRE_FLUSH,
    ] {
        assert_eq!(
            1,
            metadata.entity_listeners.get(event).unwrap().len(),
            "{event}"
        );
        assert_eq!(
            (listener_class.clone(), event.to_owned()),
            listener(&metadata, event, 0)
        );
    }
}

pub fn cache_entry(cache: Option<&PhpArray>, key: &str) -> Value {
    cache
        .and_then(|cache| cache.get(&key.into()).cloned())
        .unwrap_or(Value::Null)
}

pub fn testSecondLevelCacheMapping() {
    let factory = factory();
    let class = factory.get_metadata_for(&model("Cache\\City")).unwrap();
    let class = class.read();
    assert_eq!(
        Value::Int(cm::CACHE_USAGE_READ_ONLY),
        cache_entry(class.cache.as_ref(), "usage")
    );
    assert_eq!(
        Value::from("doctrine_tests_models_cache_city"),
        cache_entry(class.cache.as_ref(), "region")
    );

    let state = class.association_mappings.get("state").unwrap();
    assert!(state.cache.is_some());
    assert_eq!(
        Value::Int(cm::CACHE_USAGE_READ_ONLY),
        cache_entry(state.cache.as_ref(), "usage")
    );
    assert_eq!(
        Value::from("doctrine_tests_models_cache_city__state"),
        cache_entry(state.cache.as_ref(), "region")
    );

    let attractions = class.association_mappings.get("attractions").unwrap();
    assert!(attractions.cache.is_some());
    assert_eq!(
        Value::Int(cm::CACHE_USAGE_READ_ONLY),
        cache_entry(attractions.cache.as_ref(), "usage")
    );
    assert_eq!(
        Value::from("doctrine_tests_models_cache_city__attractions"),
        cache_entry(attractions.cache.as_ref(), "region")
    );
}

pub fn testSchemaDefinitionViaExplicitTableSchemaAttributeProperty() {
    let metadata = factory()
        .get_metadata_for(&model("DDC2825\\ExplicitSchemaAndTable"))
        .unwrap();
    let metadata = metadata.read();

    assert_eq!(
        Some("explicit_schema"),
        metadata.get_schema_name().as_deref()
    );
    assert_eq!("explicit_table", metadata.get_table_name().unwrap());
}

pub fn testSchemaDefinitionViaSchemaDefinedInTableNameInTableAttributeProperty() {
    let metadata = factory()
        .get_metadata_for(&model("DDC2825\\SchemaAndTableInTableName"))
        .unwrap();
    let metadata = metadata.read();

    assert_eq!(
        Some("implicit_schema"),
        metadata.get_schema_name().as_deref()
    );
    assert_eq!("implicit_table", metadata.get_table_name().unwrap());
}

pub fn discriminator_column(class: &str) -> DiscriminatorColumnMapping {
    create_class_metadata(&ns(class))
        .discriminator_column
        .unwrap()
}

pub fn testDiscriminatorColumnDefaultLength() {
    assert_eq!(
        Some(255),
        discriminator_column("SingleTableEntityNoDiscriminatorColumnMapping").length
    );
    assert_eq!(
        Some(255),
        discriminator_column("SingleTableEntityIncompleteDiscriminatorColumnMapping").length
    );
}

pub fn testDiscriminatorColumnDefaultType() {
    assert_eq!(
        "string",
        discriminator_column("SingleTableEntityNoDiscriminatorColumnMapping").r#type
    );
    assert_eq!(
        "string",
        discriminator_column("SingleTableEntityIncompleteDiscriminatorColumnMapping").r#type
    );
}

pub fn testDiscriminatorColumnDefaultName() {
    assert_eq!(
        "dtype",
        discriminator_column("SingleTableEntityNoDiscriminatorColumnMapping").name
    );
    assert_eq!(
        "dtype",
        discriminator_column("SingleTableEntityIncompleteDiscriminatorColumnMapping").name
    );
}

pub fn testReservedWordInTableColumn() {
    let metadata = create_class_metadata(&ns("ReservedWordInTableColumn"));

    assert_eq!(
        "count",
        metadata.get_field_mapping("count").unwrap().column_name
    );
}

pub fn testInsertableColumn() {
    let metadata = create_class_metadata(&model("Upsertable\\Insertable"));

    let mapping = metadata.get_field_mapping("nonInsertableContent").unwrap();

    assert_eq!(Some(cm::GENERATED_INSERT), mapping.generated);
    assert_eq!(
        None,
        metadata
            .get_field_mapping("insertableContent")
            .unwrap()
            .not_insertable
    );
}

pub fn testUpdatableColumn() {
    let metadata = create_class_metadata(&model("Upsertable\\Updatable"));

    let mapping = metadata.get_field_mapping("nonUpdatableContent").unwrap();

    assert_eq!(Some(cm::GENERATED_ALWAYS), mapping.generated);
    assert_eq!(
        None,
        metadata
            .get_field_mapping("updatableContent")
            .unwrap()
            .not_updatable
    );
}

pub fn testEnumType() {
    let metadata = create_class_metadata(&model("Enums\\Card"));

    assert_eq!(
        Some(model("Enums\\Suit")),
        metadata.field_mappings.get("suit").unwrap().enum_type
    );
}

pub fn testCustomNamingStrategyIsRespected() {
    let ns = Arc::new(crate::naming::CustomPascalNamingStrategy);
    let metadata =
        create_class_metadata_with(&self::ns("BlogPostComment"), Some(ns), None).unwrap();

    assert_eq!(Some(&"id".to_owned()), metadata.field_names.get("Id"));
    let join_column = &metadata
        .association_mappings
        .get("blogPost")
        .unwrap()
        .join_columns[0];
    assert_eq!("Id", join_column.referenced_column_name);
    assert_eq!(Some(false), join_column.nullable);
}
