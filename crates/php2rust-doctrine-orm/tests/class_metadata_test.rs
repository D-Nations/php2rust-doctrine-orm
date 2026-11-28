//! Port of `ClassMetadataTest` on the test models (tests/common/models.rs).
//! `serialize()`/`unserialize()` of the metadata are
//! `ClassMetadata::serialize` and `ClassMetadata::unserialize`; PHPUnit's
//! `assertEquals` on arrays is `phpunit_equals`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ClassMetadataTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use std::sync::Arc;

use php_value::vectors::phpunit_equals;
use php_value::{PhpArray, Thrown, Value, php_array};
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::mapping::class_metadata::{
    self as cm, ClassMetadata, ClassMetadataInfo,
};
use php2rust_doctrine_orm::mapping::naming_strategy::{
    CASE_UPPER, DefaultNamingStrategy, NamingStrategy, UnderscoreNamingStrategy,
};
use php2rust_doctrine_orm::mapping::{
    AssociationClass, AssociationKind, AssociationMapping, DiscriminatorColumnMapping,
    JoinTableMapping, mapping_exception,
};
use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

const CMS: &str = "Doctrine\\Tests\\Models\\CMS\\";
const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
const CMS_ADDRESS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddress";
const NS: &str = "Doctrine\\Tests\\ORM\\Mapping\\";

/// `new ClassMetadata($class)` and `initializeReflection()`.
fn metadata(class: &str) -> ClassMetadataInfo {
    models::register();
    let mut cm = ClassMetadataInfo::new(class, None, None);
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    cm
}

fn expect_mapping_exception<T: std::fmt::Debug>(result: Result<T, Thrown>) -> Thrown {
    let error = result.unwrap_err();
    assert!(
        thrown_is_a(&error, mapping_exception::CLASS),
        "{}: {}",
        error.class,
        error.message
    );
    error
}

fn expect_message<T: std::fmt::Debug>(result: Result<T, Thrown>, message: &str) {
    let error = expect_mapping_exception(result);
    assert!(
        text(&error).contains(message),
        "{:?} does not contain {message:?}",
        error.message
    );
}

fn text(error: &Thrown) -> String {
    String::from_utf8_lossy(error.message.as_bytes()).into_owned()
}

fn assert_array_equals(expected: &PhpArray, actual: &PhpArray) {
    assert!(
        phpunit_equals(
            &Value::Array(expected.clone()),
            &Value::Array(actual.clone())
        ),
        "{expected:?} != {actual:?}"
    );
}

fn unserialize(metadata: ClassMetadataInfo) -> ClassMetadataInfo {
    let serialized = ClassMetadata::from_info(metadata).serialize();
    ClassMetadata::unserialize(serialized.as_bytes())
        .unwrap()
        .read()
        .clone()
}

#[test]
fn testClassMetadataInstanceSerialization() {
    let mut cm = metadata(CMS_USER);

    // Test initial state
    assert!(cm.get_property_accessors().is_empty());
    assert!(cm.refl_class.is_some());
    assert_eq!(cm.name, CMS_USER);
    assert_eq!(cm.root_entity_name, CMS_USER);
    assert_eq!(cm.sub_classes, [] as [std::string::String; 0]);
    assert_eq!(cm.parent_classes, [] as [std::string::String; 0]);
    assert_eq!(cm.inheritance_type, cm::INHERITANCE_TYPE_NONE);

    // Customize state
    cm.set_inheritance_type(cm::INHERITANCE_TYPE_SINGLE_TABLE)
        .unwrap();
    cm.set_subclasses(&["One".to_owned(), "Two".to_owned(), "Three".to_owned()]);
    cm.set_parent_classes(vec!["UserParent".to_owned()]);
    cm.set_custom_repository_class(Some("UserRepository"));
    cm.set_discriminator_column(Some(php_array!["name" => "disc", "type" => "integer"]))
        .unwrap();
    cm.map_one_to_one(php_array![
        "fieldName" => "phonenumbers",
        "targetEntity" => "CmsAddress",
        "mappedBy" => "foo",
    ])
    .unwrap();
    cm.mark_read_only();
    cm.map_field(php_array![
        "fieldName" => "status",
        "notInsertable" => true,
        "generated" => cm::GENERATED_ALWAYS,
    ])
    .unwrap();

    assert!(cm.requires_fetch_after_change);
    assert_eq!(cm.association_mappings.len(), 1);

    let mut cm = unserialize(cm);
    cm.wakeup_reflection(&RuntimeReflectionService::new())
        .unwrap();

    // Check state
    assert!(!cm.get_property_accessors().is_empty());
    assert_eq!(
        cm.namespace.as_deref(),
        Some("Doctrine\\Tests\\Models\\CMS")
    );
    assert!(cm.refl_class.is_some());
    assert_eq!(cm.name, CMS_USER);
    assert_eq!(cm.root_entity_name, "UserParent");
    assert_eq!(
        cm.sub_classes,
        [
            format!("{CMS}One"),
            format!("{CMS}Two"),
            format!("{CMS}Three")
        ]
    );
    assert_eq!(cm.parent_classes, ["UserParent"]);
    assert_eq!(
        cm.custom_repository_class_name.as_deref(),
        Some("Doctrine\\Tests\\Models\\CMS\\UserRepository")
    );
    assert_eq!(
        cm.discriminator_column.as_ref(),
        Some(
            &DiscriminatorColumnMapping::from_mapping_array(&php_array![
                "name" => "disc",
                "type" => "integer",
                "fieldName" => "disc",
            ])
            .unwrap()
        )
    );
    assert!(
        cm.association_mappings
            .get("phonenumbers")
            .unwrap()
            .is_one_to_one()
    );
    assert_eq!(cm.association_mappings.len(), 1);
    let one_one_mapping = cm.get_association_mapping("phonenumbers").unwrap();
    assert_eq!(one_one_mapping.fetch, Some(cm::FETCH_LAZY));
    assert_eq!(one_one_mapping.field_name, "phonenumbers");
    assert_eq!(one_one_mapping.target_entity, CMS_ADDRESS);
    assert!(cm.is_read_only);
    assert!(cm.requires_fetch_after_change);
}

#[test]
fn testFieldIsNullable() {
    let mut cm = metadata(CMS_USER);

    // Explicit Nullable
    cm.map_field(php_array!["fieldName" => "status", "nullable" => true, "type" => "string", "length" => 50_i64])
        .unwrap();
    assert!(cm.is_nullable("status").unwrap());

    // Explicit Not Nullable
    cm.map_field(php_array!["fieldName" => "username", "nullable" => false, "type" => "string", "length" => 50_i64])
        .unwrap();
    assert!(!cm.is_nullable("username").unwrap());

    // Implicit Not Nullable
    cm.map_field(php_array!["fieldName" => "name", "type" => "string", "length" => 50_i64])
        .unwrap();
    assert!(
        !cm.is_nullable("name").unwrap(),
        "By default a field should not be nullable."
    );
}

const TYPED: &str = "Doctrine\\Tests\\Models\\TypedProperties\\";

#[test]
fn testFieldIsNullableByType() {
    let mut cm = metadata(&format!("{TYPED}UserTyped"));

    cm.map_one_to_one(php_array![
        "fieldName" => "email",
        "joinColumns" => php_array![php_array!["name" => "email_id", "referencedColumnName" => "id"]],
    ])
    .unwrap();
    assert_eq!(
        cm.get_association_mapping("email").unwrap().target_entity,
        "Doctrine\\Tests\\Models\\CMS\\CmsEmail"
    );

    cm.map_many_to_one(php_array!["fieldName" => "mainEmail"])
        .unwrap();
    assert_eq!(
        cm.get_association_mapping("mainEmail")
            .unwrap()
            .target_entity,
        "Doctrine\\Tests\\Models\\CMS\\CmsEmail"
    );

    cm.map_embedded(php_array!["fieldName" => "contact"])
        .unwrap();
    assert_eq!(
        cm.embedded_classes.get("contact").unwrap().class,
        format!("{TYPED}Contact")
    );
}

#[test]
fn testFieldTypeFromReflection() {
    let mut cm = metadata(&format!("{TYPED}UserTyped"));
    let cases: [(&str, Option<i64>, &str); 9] = [
        ("id", None, "integer"),
        ("username", Some(50), "string"),
        ("dateInterval", None, "dateinterval"),
        ("dateTime", None, "datetime"),
        ("dateTimeImmutable", None, "datetime_immutable"),
        ("array", None, "json"),
        ("boolean", None, "boolean"),
        ("float", None, "float"),
        // number, requires DBAL 4.3+
        ("bodyHeight", None, "number"),
    ];
    for (field, length, ty) in cases {
        let mut mapping = php_array!["fieldName" => field];
        if let Some(length) = length {
            mapping.insert("length", Value::Int(length));
        }
        cm.map_field(mapping).unwrap();
        assert_eq!(cm.get_type_of_field(field).as_deref(), Some(ty), "{field}");
    }
}

#[derive(Debug)]
struct CustomIntAsStringTypedFieldMapper;

impl php2rust_doctrine_orm::mapping::typed_field_mapper::TypedFieldMapper
    for CustomIntAsStringTypedFieldMapper
{
    fn validate_and_complete(
        &self,
        mut mapping: PhpArray,
        field: &php2rust_doctrine_persistence::php_reflection::ReflectionProperty,
    ) -> Result<PhpArray, Thrown> {
        let ty = field.get_type();
        if mapping.get(&"type".into()).is_none() && ty.is_some_and(|ty| ty.get_name() == "int") {
            mapping.insert("type", Value::from("string"));
        }
        Ok(mapping)
    }
}

#[test]
fn testFieldTypeFromReflectionDefaultTypedFieldMapper() {
    use php2rust_doctrine_orm::mapping::typed_field_mapper::DefaultTypedFieldMapper;
    models::register();
    let custom_id_object_type = "Doctrine\\Tests\\DbalTypes\\CustomIdObjectType";
    let custom_int_type = "Doctrine\\Tests\\DbalTypes\\CustomIntType";
    let mut cm = ClassMetadataInfo::new(
        format!("{TYPED}UserTypedWithCustomTypedField"),
        None,
        Some(Arc::new(DefaultTypedFieldMapper::new(vec![
            (
                "Doctrine\\Tests\\DbalTypes\\CustomIdObject".to_owned(),
                custom_id_object_type.to_owned(),
            ),
            ("int".to_owned(), custom_int_type.to_owned()),
        ]))),
    );
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();

    cm.map_field(php_array!["fieldName" => "customId"]).unwrap();
    cm.map_field(php_array!["fieldName" => "customIntTypedField"])
        .unwrap();
    assert_eq!(
        cm.get_type_of_field("customId").as_deref(),
        Some(custom_id_object_type)
    );
    assert_eq!(
        cm.get_type_of_field("customIntTypedField").as_deref(),
        Some(custom_int_type)
    );
}

#[test]
fn testFieldTypeFromReflectionChainTypedFieldMapper() {
    use php2rust_doctrine_orm::mapping::typed_field_mapper::{
        ChainTypedFieldMapper, DefaultTypedFieldMapper,
    };
    models::register();
    let mut cm = ClassMetadataInfo::new(
        format!("{TYPED}UserTyped"),
        None,
        Some(Arc::new(ChainTypedFieldMapper::new(vec![
            Arc::new(CustomIntAsStringTypedFieldMapper),
            Arc::new(DefaultTypedFieldMapper::default()),
        ]))),
    );
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();

    cm.map_field(php_array!["fieldName" => "id"]).unwrap();
    cm.map_field(php_array!["fieldName" => "username"]).unwrap();
    assert_eq!(cm.get_type_of_field("id").as_deref(), Some("string"));
    assert_eq!(cm.get_type_of_field("username").as_deref(), Some("string"));
}

#[test]
fn testMapAssociationInGlobalNamespace() {
    let mut cm = metadata("DoctrineGlobalArticle");
    cm.map_many_to_many(php_array![
        "fieldName" => "author",
        "targetEntity" => "DoctrineGlobalUser",
        "joinTable" => php_array![
            "name" => "bar",
            "joinColumns" => php_array![php_array!["name" => "bar_id", "referencedColumnName" => "id"]],
            "inverseJoinColumns" => php_array![php_array!["name" => "baz_id", "referencedColumnName" => "id"]],
        ],
    ])
    .unwrap();

    assert_eq!(
        cm.association_mappings.get("author").unwrap().target_entity,
        "DoctrineGlobalUser"
    );
}

#[test]
fn testMapManyToManyJoinTableDefaults() {
    let mut cm = metadata(CMS_USER);
    cm.map_many_to_many(php_array!["fieldName" => "groups", "targetEntity" => "CmsGroup"])
        .unwrap();

    let assoc = cm.association_mappings.get("groups").unwrap();
    // assertEquals(): `nullable` false equals null.
    let expected = JoinTableMapping::from_mapping_array(&php_array![
        "name" => "cmsuser_cmsgroup",
        "joinColumns" => php_array![php_array!["name" => "cmsuser_id", "referencedColumnName" => "id", "onDelete" => "CASCADE"]],
        "inverseJoinColumns" => php_array![php_array!["name" => "cmsgroup_id", "referencedColumnName" => "id", "onDelete" => "CASCADE"]],
    ])
    .unwrap();
    let actual = assoc.join_table.as_ref().unwrap();
    assert_eq!(
        (
            &expected.name,
            &expected.schema,
            expected.quoted,
            &expected.options
        ),
        (&actual.name, &actual.schema, actual.quoted, &actual.options)
    );
    let columns = |table: &JoinTableMapping| {
        Value::Array(PhpArray::list(
            table
                .join_columns
                .iter()
                .chain(&table.inverse_join_columns)
                .map(|column| Value::Array(column.to_array())),
        ))
    };
    assert!(phpunit_equals(&columns(&expected), &columns(actual)));
    assert_eq!(assoc.is_on_delete_cascade, Some(true));
}

#[test]
fn testSerializeManyToManyJoinTableCascade() {
    let mut cm = metadata(CMS_USER);
    cm.map_many_to_many(php_array!["fieldName" => "groups", "targetEntity" => "CmsGroup"])
        .unwrap();

    let assoc = cm.association_mappings.get("groups").unwrap();
    let serialized = php2rust_doctrine_orm::mapping::serialize_object(assoc);
    let assoc = AssociationMapping::unserialize(serialized.as_bytes(), None).unwrap();

    assert_eq!(assoc.is_on_delete_cascade, Some(true));
}

#[test]
fn testSetDiscriminatorMapInGlobalNamespace() {
    let mut cm = metadata("DoctrineGlobalUser");
    cm.set_discriminator_map(
        &php_array!["descr" => "DoctrineGlobalArticle", "foo" => "DoctrineGlobalUser"],
    )
    .unwrap();

    assert!(
        cm.discriminator_map
            .get(&"descr".into())
            .unwrap()
            .identical(&Value::from("DoctrineGlobalArticle"))
    );
    assert!(
        cm.discriminator_map
            .get(&"foo".into())
            .unwrap()
            .identical(&Value::from("DoctrineGlobalUser"))
    );
}

#[test]
fn testSetSubClassesInGlobalNamespace() {
    let mut cm = metadata("DoctrineGlobalUser");
    cm.set_subclasses(&["DoctrineGlobalArticle".to_owned()]);

    assert_eq!(cm.sub_classes[0], "DoctrineGlobalArticle");
}

#[test]
fn testSetInvalidVersionMappingThrowsException() {
    let mut field = php_array!["fieldName" => "foo", "type" => "string"];
    let mut cm = metadata(CMS_USER);

    expect_mapping_exception(cm.set_version_mapping(&mut field));
}

#[test]
fn testItThrowsOnJoinColumnForInverseOneToOne() {
    let mut cm = metadata(CMS_USER);

    expect_message(
        cm.map_one_to_one(php_array![
            "fieldName" => "address",
            "targetEntity" => CMS_ADDRESS,
            "mappedBy" => "user",
            "joinColumns" => php_array!["non", "empty", "list"],
        ]),
        "Doctrine\\Tests\\Models\\CMS\\CmsUser#address is a OneToOne inverse side, which does not allow join columns.",
    );
}

#[test]
fn testGetSingleIdentifierFieldNameMultipleIdentifierEntityThrowsException() {
    let mut cm = metadata(CMS_USER);
    cm.is_identifier_composite = true;

    expect_mapping_exception(cm.get_single_identifier_field_name());
}

#[test]
fn testGetSingleIdentifierFieldNameNoIdEntityThrowsException() {
    let cm = metadata("Doctrine\\Tests\\Models\\DDC6412\\DDC6412File");

    expect_mapping_exception(cm.get_single_identifier_field_name());
}

#[test]
fn testDuplicateAssociationMappingException() {
    let mut cm = metadata(CMS_USER);
    let one_to_many = || {
        AssociationMapping::from_mapping_array(
            AssociationClass::Final(AssociationKind::OneToMany),
            &php_array![
                "fieldName" => "foo",
                "sourceEntity" => "stdClass",
                "targetEntity" => "stdClass",
                "mappedBy" => "foo",
            ],
        )
        .unwrap()
    };

    cm.add_inherited_association_mapping(one_to_many()).unwrap();
    expect_mapping_exception(cm.add_inherited_association_mapping(one_to_many()));
}

#[test]
fn testDuplicateColumnNameThrowsMappingException() {
    let mut cm = metadata(CMS_USER);
    cm.map_field(php_array!["fieldName" => "name", "columnName" => "name"])
        .unwrap();

    expect_mapping_exception(
        cm.map_field(php_array!["fieldName" => "username", "columnName" => "name"]),
    );
}

#[test]
fn testDuplicateColumnNameDiscriminatorColumnThrowsMappingException() {
    let mut cm = metadata(CMS_USER);
    cm.map_field(php_array!["fieldName" => "name", "columnName" => "name"])
        .unwrap();

    expect_mapping_exception(cm.set_discriminator_column(Some(php_array!["name" => "name"])));
}

#[test]
fn testDuplicateColumnNameDiscriminatorColumn2ThrowsMappingException() {
    let mut cm = metadata(CMS_USER);
    cm.set_discriminator_column(Some(php_array!["name" => "name"]))
        .unwrap();

    expect_mapping_exception(
        cm.map_field(php_array!["fieldName" => "name", "columnName" => "name"]),
    );
}

#[test]
fn testDuplicateFieldAndAssociationMapping1ThrowsException() {
    let mut cm = metadata(CMS_USER);
    cm.map_field(php_array!["fieldName" => "name", "columnName" => "name"])
        .unwrap();

    expect_mapping_exception(
        cm.map_one_to_one(php_array!["fieldName" => "name", "targetEntity" => "CmsUser"]),
    );
}

#[test]
fn testDuplicateFieldAndAssociationMapping2ThrowsException() {
    let mut cm = metadata(CMS_USER);
    cm.map_one_to_one(php_array!["fieldName" => "name", "targetEntity" => "CmsUser"])
        .unwrap();

    expect_mapping_exception(
        cm.map_field(php_array!["fieldName" => "name", "columnName" => "name"]),
    );
}

#[test]
fn testGetTemporaryTableNameSchema() {
    let mut cm = metadata(CMS_USER);
    cm.set_table_name("foo.bar");

    assert_eq!(cm.get_temporary_id_table_name().unwrap(), "foo_bar_id_tmp");
}

#[test]
fn testDefaultTableName() {
    let mut cm = metadata(CMS_USER);

    // When table's name is not given
    cm.set_primary_table(&PhpArray::new());

    assert_eq!(cm.get_table_name().unwrap(), "CmsUser");
    assert!(
        cm.table
            .get(&"name".into())
            .unwrap()
            .identical(&Value::from("CmsUser"))
    );

    let mut cm = metadata(CMS_ADDRESS);
    // When joinTable's name is not given
    cm.map_many_to_many(php_array![
        "fieldName" => "user",
        "targetEntity" => "CmsUser",
        "inversedBy" => "users",
        "joinTable" => php_array![
            "joinColumns" => php_array![php_array!["referencedColumnName" => "id"]],
            "inverseJoinColumns" => php_array![php_array!["referencedColumnName" => "id"]],
        ],
    ])
    .unwrap();
    assert_eq!(
        cm.association_mappings
            .get("user")
            .unwrap()
            .join_table
            .as_ref()
            .unwrap()
            .name,
        "cmsaddress_cmsuser"
    );
}

#[test]
fn testDefaultJoinColumnName() {
    let mut cm = metadata(CMS_ADDRESS);

    // this is really dirty, but it's the simplest way to test whether
    // joinColumn's name will be automatically set to user_id
    cm.map_one_to_one(php_array![
        "fieldName" => "user",
        "targetEntity" => "CmsUser",
        "joinColumns" => php_array![php_array!["referencedColumnName" => "id"]],
    ])
    .unwrap();
    assert_eq!(
        cm.association_mappings.get("user").unwrap().join_columns[0].name,
        "user_id"
    );

    let mut cm = metadata(CMS_ADDRESS);
    cm.map_many_to_many(php_array![
        "fieldName" => "user",
        "targetEntity" => "CmsUser",
        "inversedBy" => "users",
        "joinTable" => php_array![
            "name" => "user_CmsUser",
            "joinColumns" => php_array![php_array!["referencedColumnName" => "id"]],
            "inverseJoinColumns" => php_array![php_array!["referencedColumnName" => "id"]],
        ],
    ])
    .unwrap();
    let join_table = cm
        .association_mappings
        .get("user")
        .unwrap()
        .join_table
        .clone()
        .unwrap();
    assert_eq!(join_table.join_columns[0].name, "cmsaddress_id");
    assert_eq!(join_table.inverse_join_columns[0].name, "cmsuser_id");
}

#[test]
fn testUnderscoreNamingStrategyDefaults() {
    models::register();
    let naming_strategy = Arc::new(UnderscoreNamingStrategy::new(CASE_UPPER));
    let mut one_to_one_metadata =
        ClassMetadataInfo::new(CMS_ADDRESS, Some(naming_strategy.clone()), None);
    let mut many_to_many_metadata =
        ClassMetadataInfo::new(CMS_ADDRESS, Some(naming_strategy.clone()), None);

    one_to_one_metadata
        .map_one_to_one(php_array!["fieldName" => "user", "targetEntity" => "CmsUser"])
        .unwrap();
    many_to_many_metadata
        .map_many_to_many(php_array!["fieldName" => "user", "targetEntity" => "CmsUser"])
        .unwrap();

    let one = one_to_one_metadata
        .association_mappings
        .get("user")
        .unwrap();
    assert_array_equals(
        &php_array!["USER_ID" => "ID"],
        &one.source_to_target_key_columns,
    );
    assert_array_equals(
        &php_array!["USER_ID" => "USER_ID"],
        &one.join_column_field_names,
    );
    assert_array_equals(
        &php_array!["ID" => "USER_ID"],
        &one.target_to_source_key_columns,
    );
    assert_eq!(one.join_columns[0].name, "USER_ID");
    assert_eq!(one.join_columns[0].referenced_column_name, "ID");

    let many = many_to_many_metadata
        .association_mappings
        .get("user")
        .unwrap();
    let join_table = many.join_table.as_ref().unwrap();
    assert_eq!(join_table.name, "CMS_ADDRESS_CMS_USER");
    assert_eq!(many.join_table_columns, ["CMS_ADDRESS_ID", "CMS_USER_ID"]);
    assert_array_equals(
        &php_array!["CMS_ADDRESS_ID" => "ID"],
        &many.relation_to_source_key_columns,
    );
    assert_array_equals(
        &php_array!["CMS_USER_ID" => "ID"],
        &many.relation_to_target_key_columns,
    );
    assert_eq!(join_table.join_columns[0].name, "CMS_ADDRESS_ID");
    assert_eq!(join_table.inverse_join_columns[0].name, "CMS_USER_ID");
    assert_eq!(join_table.join_columns[0].referenced_column_name, "ID");
    assert_eq!(
        join_table.inverse_join_columns[0].referenced_column_name,
        "ID"
    );

    let mut cm = ClassMetadataInfo::new("DoctrineGlobalArticle", Some(naming_strategy), None);
    cm.map_many_to_many(php_array!["fieldName" => "author", "targetEntity" => CMS_USER])
        .unwrap();
    assert_eq!(
        cm.association_mappings
            .get("author")
            .unwrap()
            .join_table
            .as_ref()
            .unwrap()
            .name,
        "DOCTRINE_GLOBAL_ARTICLE_CMS_USER"
    );
}

#[test]
fn testSetMultipleIdentifierSetsComposite() {
    let mut cm = metadata(CMS_USER);
    cm.map_field(php_array!["fieldName" => "name"]).unwrap();
    cm.map_field(php_array!["fieldName" => "username"]).unwrap();

    cm.set_identifier(&["name".to_owned(), "username".to_owned()], &[]);
    assert!(cm.is_identifier_composite);
}

#[test]
fn testMappingNotFound() {
    let cm = metadata(CMS_USER);

    let error = expect_mapping_exception(cm.get_field_mapping("foo"));
    assert_eq!(
        text(&error),
        format!("No mapping found for field 'foo' on class '{CMS_USER}'.")
    );
}

#[test]
fn testJoinTableMappingDefaults() {
    let mut cm = metadata("DoctrineGlobalArticle");
    cm.map_many_to_many(php_array!["fieldName" => "author", "targetEntity" => CMS_USER])
        .unwrap();

    assert_eq!(
        cm.association_mappings
            .get("author")
            .unwrap()
            .join_table
            .as_ref()
            .unwrap()
            .name,
        "doctrineglobalarticle_cmsuser"
    );
}

const DDC117: &str = "Doctrine\\Tests\\Models\\DDC117\\";

#[test]
fn testMapIdentifierAssociation() {
    let mut cm = metadata(&format!("{DDC117}DDC117ArticleDetails"));
    cm.map_one_to_one(php_array![
        "fieldName" => "article",
        "id" => true,
        "targetEntity" => format!("{DDC117}DDC117Article").as_str(),
        "joinColumns" => PhpArray::new(),
    ])
    .unwrap();

    assert!(
        cm.contains_foreign_identifier,
        "Identifier Association should set 'containsForeignIdentifier' boolean flag."
    );
    assert_eq!(cm.identifier, ["article"]);
}

#[test]
fn testOrphanRemovalIdentifierAssociation() {
    let mut cm = metadata(&format!("{DDC117}DDC117ArticleDetails"));
    expect_message(
        cm.map_one_to_one(php_array![
            "fieldName" => "article",
            "id" => true,
            "targetEntity" => format!("{DDC117}DDC117Article").as_str(),
            "orphanRemoval" => true,
            "joinColumns" => PhpArray::new(),
        ]),
        "The orphan removal option is not allowed on an association that",
    );
}

#[test]
fn testInverseIdentifierAssociation() {
    let mut cm = metadata(&format!("{DDC117}DDC117ArticleDetails"));
    expect_message(
        cm.map_one_to_one(php_array![
            "fieldName" => "article",
            "id" => true,
            "mappedBy" => "details",
            "targetEntity" => format!("{DDC117}DDC117Article").as_str(),
            "joinColumns" => PhpArray::new(),
        ]),
        "An inverse association is not allowed to be identifier in",
    );
}

#[test]
fn testIdentifierAssociationManyToMany() {
    let mut cm = metadata(&format!("{DDC117}DDC117ArticleDetails"));
    expect_message(
        cm.map_many_to_many(php_array![
            "fieldName" => "article",
            "id" => true,
            "targetEntity" => format!("{DDC117}DDC117Article").as_str(),
            "joinColumns" => PhpArray::new(),
        ]),
        "Many-to-many or one-to-many associations are not allowed to be identifier in",
    );
}

#[test]
fn testEmptyFieldNameThrowsException() {
    let mut cm = metadata(CMS_USER);
    expect_message(
        cm.map_field(php_array!["fieldName" => ""]),
        &format!(
            "The field or association mapping misses the 'fieldName' attribute in entity '{CMS_USER}'."
        ),
    );
}

#[test]
fn testSerializeEntityListeners() {
    let mut metadata = metadata("Doctrine\\Tests\\Models\\Company\\CompanyContract");
    metadata
        .add_entity_listener(
            events::PRE_PERSIST,
            "CompanyContractListener",
            "prePersistHandler",
        )
        .unwrap();
    metadata
        .add_entity_listener(
            events::POST_PERSIST,
            "CompanyContractListener",
            "postPersistHandler",
        )
        .unwrap();

    let unserialized = unserialize(metadata.clone());

    assert_eq!(metadata.entity_listeners, unserialized.entity_listeners);
}

#[test]
fn testClassCaseSensitivity() {
    let cm = metadata(&CMS_USER.to_ascii_uppercase());

    assert_eq!(cm.name, CMS_USER);
}

#[test]
fn testLifecycleCallbackNotFound() {
    let mut cm = metadata(CMS_USER);
    cm.add_lifecycle_callback("notfound", "postLoad").unwrap();

    let error =
        expect_mapping_exception(cm.validate_lifecycle_callbacks(&RuntimeReflectionService::new()));
    assert_eq!(
        text(&error),
        format!(
            "Entity '{CMS_USER}' has no method 'notfound' to be registered as lifecycle callback."
        )
    );
}

#[test]
fn testTargetEntityNotFound() {
    let mut cm = metadata(CMS_USER);
    cm.map_many_to_one(php_array!["fieldName" => "address", "targetEntity" => "UnknownClass"])
        .unwrap();

    let error = expect_mapping_exception(cm.validate_associations());
    assert_eq!(
        text(&error),
        format!(
            "The target-entity Doctrine\\Tests\\Models\\CMS\\UnknownClass cannot be found in '{CMS_USER}#address'."
        )
    );
}

#[test]
fn testNameIsMandatoryForDiscriminatorColumnsMappingException() {
    let mut cm = metadata(CMS_USER);
    let error = expect_mapping_exception(cm.set_discriminator_column(Some(PhpArray::new())));
    assert_eq!(
        text(&error),
        "Discriminator column name on entity class 'Doctrine\\Tests\\Models\\CMS\\CmsUser' is not defined."
    );
}

/// `MyNamespacedNamingStrategy`: `DefaultNamingStrategy` with the
/// namespace in the table name (its `classToTableName()` is what the
/// default's other methods call).
#[derive(Debug, Default)]
struct MyNamespacedNamingStrategy;

impl NamingStrategy for MyNamespacedNamingStrategy {
    fn class_to_table_name(&self, class_name: &str) -> String {
        let mut class_name = class_name.to_owned();
        if class_name.contains('\\') {
            class_name = class_name
                .replace("Doctrine\\Tests\\Models\\", "")
                .replace('\\', "_");
        }
        class_name.to_ascii_lowercase()
    }

    fn property_to_column_name(&self, property_name: &str, _: &str) -> String {
        property_name.to_owned()
    }

    fn embedded_field_to_column_name(
        &self,
        property: &str,
        column: &str,
        _: &str,
        _: &str,
    ) -> String {
        format!("{property}_{column}")
    }

    fn reference_column_name(&self) -> String {
        "id".to_owned()
    }

    fn join_column_name(&self, property_name: &str, _: &str) -> String {
        format!("{property_name}_{}", self.reference_column_name())
    }

    fn join_table_name(&self, source: &str, target: &str, _: &str) -> String {
        format!(
            "{}_{}",
            self.class_to_table_name(source),
            self.class_to_table_name(target)
        )
        .to_ascii_lowercase()
    }

    fn join_key_column_name(&self, entity_name: &str, referenced: Option<&str>) -> String {
        let referenced = referenced
            .filter(|r| !r.is_empty() && *r != "0")
            .map_or_else(|| self.reference_column_name(), str::to_owned);
        format!("{}_{referenced}", self.class_to_table_name(entity_name)).to_ascii_lowercase()
    }
}

/// `MyPrefixNamingStrategy`: the table name before each column name.
#[derive(Debug, Default)]
struct MyPrefixNamingStrategy(DefaultNamingStrategy);

impl NamingStrategy for MyPrefixNamingStrategy {
    fn class_to_table_name(&self, class_name: &str) -> String {
        self.0.class_to_table_name(class_name)
    }

    fn property_to_column_name(&self, property_name: &str, class_name: &str) -> String {
        format!(
            "{}_{property_name}",
            self.class_to_table_name(class_name).to_ascii_lowercase()
        )
    }

    fn embedded_field_to_column_name(&self, a: &str, b: &str, c: &str, d: &str) -> String {
        self.0.embedded_field_to_column_name(a, b, c, d)
    }

    fn reference_column_name(&self) -> String {
        self.0.reference_column_name()
    }

    fn join_column_name(&self, property_name: &str, class_name: &str) -> String {
        self.0.join_column_name(property_name, class_name)
    }

    fn join_table_name(&self, source: &str, target: &str, property: &str) -> String {
        self.0.join_table_name(source, target, property)
    }

    fn join_key_column_name(&self, entity_name: &str, referenced: Option<&str>) -> String {
        self.0.join_key_column_name(entity_name, referenced)
    }
}

fn with_strategy(class: &str, naming: Arc<dyn NamingStrategy>) -> ClassMetadataInfo {
    models::register();
    let mut cm = ClassMetadataInfo::new(class, Some(naming), None);
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    cm
}

#[test]
fn testFullyQualifiedClassNameShouldBeGivenToNamingStrategy() {
    let naming: Arc<dyn NamingStrategy> = Arc::new(MyNamespacedNamingStrategy);
    let mut address_metadata = with_strategy(CMS_ADDRESS, naming.clone());
    let mut article_metadata = with_strategy("DoctrineGlobalArticle", naming.clone());
    let routing_metadata = with_strategy("Doctrine\\Tests\\Models\\Routing\\RoutingLeg", naming);

    address_metadata
        .map_many_to_many(php_array!["fieldName" => "user", "targetEntity" => "CmsUser"])
        .unwrap();
    article_metadata
        .map_many_to_many(php_array!["fieldName" => "author", "targetEntity" => CMS_USER])
        .unwrap();

    assert!(
        routing_metadata
            .table
            .get(&"name".into())
            .unwrap()
            .identical(&Value::from("routing_routingleg"))
    );
    let join_table_name = |cm: &ClassMetadataInfo, field: &str| {
        cm.association_mappings
            .get(field)
            .unwrap()
            .join_table
            .as_ref()
            .unwrap()
            .name
            .clone()
    };
    assert_eq!(
        join_table_name(&address_metadata, "user"),
        "cms_cmsaddress_cms_cmsuser"
    );
    assert_eq!(
        join_table_name(&article_metadata, "author"),
        "doctrineglobalarticle_cms_cmsuser"
    );
}

#[test]
fn testFullyQualifiedClassNameShouldBeGivenToNamingStrategyPropertyToColumnName() {
    let mut metadata = with_strategy(CMS_ADDRESS, Arc::new(MyPrefixNamingStrategy::default()));

    metadata
        .map_field(php_array!["fieldName" => "country"])
        .unwrap();
    metadata
        .map_field(php_array!["fieldName" => "city"])
        .unwrap();

    let field_names: Vec<(&str, &str)> = metadata
        .field_names
        .iter()
        .map(|(column, field)| (column, field.as_str()))
        .collect();
    assert_eq!(
        field_names,
        [
            ("cmsaddress_country", "country"),
            ("cmsaddress_city", "city")
        ]
    );
}

#[test]
fn testInvalidCascade() {
    let mut cm = metadata(CMS_USER);
    expect_message(
        cm.map_many_to_one(php_array![
            "fieldName" => "address",
            "targetEntity" => "UnknownClass",
            "cascade" => php_array!["invalid"],
        ]),
        &format!(
            "You have specified invalid cascade options for {CMS_USER}::$address: 'invalid'; available options: 'remove', 'persist', 'refresh', and 'detach'"
        ),
    );
}

const DDC964: &str = "Doctrine\\Tests\\Models\\DDC964\\";

#[test]
fn testInvalidPropertyAssociationOverrideNameException() {
    let mut cm = metadata(&format!("{DDC964}DDC964Admin"));
    cm.map_many_to_one(php_array!["fieldName" => "address", "targetEntity" => "DDC964Address"])
        .unwrap();

    expect_message(
        cm.set_association_override("invalidPropertyName", &PhpArray::new()),
        "Invalid field override named 'invalidPropertyName' for class 'Doctrine\\Tests\\Models\\DDC964\\DDC964Admin",
    );
}

#[test]
fn testInvalidPropertyAttributeOverrideNameException() {
    let mut cm = metadata(&format!("{DDC964}DDC964Guest"));
    cm.map_field(php_array!["fieldName" => "name"]).unwrap();

    expect_message(
        cm.set_attribute_override("invalidPropertyName", &PhpArray::new()),
        "Invalid field override named 'invalidPropertyName' for class 'Doctrine\\Tests\\Models\\DDC964\\DDC964Guest'.",
    );
}

#[test]
fn testInvalidOverrideAttributeFieldTypeException() {
    let mut cm = metadata(&format!("{DDC964}DDC964Guest"));
    cm.map_field(php_array!["fieldName" => "name", "type" => "string"])
        .unwrap();

    expect_message(
        cm.set_attribute_override("name", &php_array!["type" => "date"]),
        "The column type of attribute 'name' on class 'Doctrine\\Tests\\Models\\DDC964\\DDC964Guest' could not be changed.",
    );
}

const DIRECTORY: &str = "Doctrine\\Tests\\Models\\DirectoryTree\\Directory";
const CONTENT_ITEM: &str = "Doctrine\\Tests\\Models\\DirectoryTree\\AbstractContentItem";

#[test]
fn testAttributeOverrideKeepsDeclaringClass() {
    models::register();
    let mut cm = ClassMetadataInfo::new(DIRECTORY, None, None);
    cm.map_field(php_array!["fieldName" => "id", "type" => "integer", "declared" => CONTENT_ITEM])
        .unwrap();
    cm.set_attribute_override("id", &php_array!["columnName" => "new_id"])
        .unwrap();

    let mapping = cm.get_field_mapping("id").unwrap();

    assert_eq!(mapping.declared.as_deref(), Some(CONTENT_ITEM));
}

fn parent_directory(cascade: &str) -> AssociationMapping {
    models::register();
    let mut cm = ClassMetadataInfo::new(DIRECTORY, None, None);
    cm.map_many_to_one(php_array![
        "fieldName" => "parentDirectory",
        "targetEntity" => DIRECTORY,
        "cascade" => php_array!["remove"],
        "declared" => DIRECTORY,
    ])
    .unwrap();
    cm.set_association_override(
        "parentDirectory",
        &php_array!["cascade" => php_array![cascade]],
    )
    .unwrap();
    cm.get_association_mapping("parentDirectory")
        .unwrap()
        .clone()
}

#[test]
fn testAssociationOverrideKeepsDeclaringClass() {
    let mapping = parent_directory("remove");

    assert_eq!(mapping.declared.as_deref(), Some(DIRECTORY));
}

#[test]
fn testAssociationOverrideCanOverrideCascade() {
    let mapping = parent_directory("all");

    assert_eq!(mapping.cascade, ["remove", "persist", "refresh", "detach"]);
}

#[test]
fn testInvalidEntityListenerClassException() {
    let mut cm = metadata(CMS_USER);
    let error = expect_mapping_exception(cm.add_entity_listener(
        events::POST_LOAD,
        "\\InvalidClassName",
        "postLoadHandler",
    ));
    assert_eq!(
        text(&error),
        "Entity Listener \"\\InvalidClassName\" declared on \"Doctrine\\Tests\\Models\\CMS\\CmsUser\" not found."
    );
}

#[test]
fn testInvalidEntityListenerMethodException() {
    let mut cm = metadata(CMS_USER);
    let error = expect_mapping_exception(cm.add_entity_listener(
        events::POST_LOAD,
        "Doctrine\\Tests\\Models\\Company\\CompanyContractListener",
        "invalidMethod",
    ));
    assert_eq!(
        text(&error),
        "Entity Listener \"Doctrine\\Tests\\Models\\Company\\CompanyContractListener\" declared on \"Doctrine\\Tests\\Models\\CMS\\CmsUser\" has no method \"invalidMethod\"."
    );
}

#[test]
fn testManyToManySelfReferencingNamingStrategyDefaults() {
    let mut cm = metadata("Doctrine\\Tests\\Models\\CustomType\\CustomTypeParent");
    cm.map_many_to_many(
        php_array!["fieldName" => "friendsWithMe", "targetEntity" => "CustomTypeParent"],
    )
    .unwrap();

    let assoc = cm.association_mappings.get("friendsWithMe").unwrap();
    let join_table = assoc.join_table.as_ref().unwrap();
    assert_eq!(join_table.name, "customtypeparent_customtypeparent");
    let columns = |columns: &[php2rust_doctrine_orm::mapping::JoinColumnMapping]| {
        Value::Array(PhpArray::list(
            columns.iter().map(|c| Value::Array(c.to_array())),
        ))
    };
    let expected = JoinTableMapping::from_mapping_array(&php_array![
        "name" => "customtypeparent_customtypeparent",
        "joinColumns" => php_array![php_array!["name" => "customtypeparent_source", "referencedColumnName" => "id", "onDelete" => "CASCADE"]],
        "inverseJoinColumns" => php_array![php_array!["name" => "customtypeparent_target", "referencedColumnName" => "id", "onDelete" => "CASCADE"]],
    ])
    .unwrap();
    // assertEquals(): `nullable` false equals null.
    assert!(phpunit_equals(
        &columns(&expected.join_columns),
        &columns(&join_table.join_columns)
    ));
    assert!(phpunit_equals(
        &columns(&expected.inverse_join_columns),
        &columns(&join_table.inverse_join_columns)
    ));
    assert_eq!(
        assoc.join_table_columns,
        ["customtypeparent_source", "customtypeparent_target"]
    );
    assert_array_equals(
        &php_array!["customtypeparent_source" => "id"],
        &assoc.relation_to_source_key_columns,
    );
    assert_array_equals(
        &php_array!["customtypeparent_target" => "id"],
        &assoc.relation_to_target_key_columns,
    );
}

#[test]
fn testSetSequenceGeneratorThrowsExceptionWhenSequenceNameIsMissing() {
    let mut cm = metadata(CMS_USER);

    expect_mapping_exception(cm.set_sequence_generator_definition(PhpArray::new()));
}

#[test]
fn testQuotedSequenceName() {
    let mut cm = metadata(CMS_USER);
    cm.set_sequence_generator_definition(php_array!["sequenceName" => "`foo`"])
        .unwrap();

    assert!(
        Value::Array(cm.sequence_generator_definition.clone().unwrap()).identical(&Value::Array(
            php_array!["sequenceName" => "foo", "quoted" => true, "allocationSize" => "1", "initialValue" => "1"]
        ))
    );
}

const TEST_CLASS: &str = "Doctrine\\Tests\\ORM\\Mapping\\ClassMetadataTest";

#[test]
fn testGetSequenceName() {
    // fullTableNameProvider
    let cases = [
        (
            "quoted table name with schema",
            "custom.reserved_id_seq",
            "custom.`reserved`",
        ),
        (
            "unquoted table name with schema",
            "custom.non_reserved_id_seq",
            "custom.non_reserved",
        ),
        (
            "quoted table name without schema",
            "reserved_id_seq",
            "`reserved`",
        ),
        (
            "unquoted table name without schema",
            "non_reserved_id_seq",
            "non_reserved",
        ),
    ];
    for (name, expected_sequence_name, full_table_name) in cases {
        let mut cm = ClassMetadataInfo::new(TEST_CLASS, None, None);
        cm.set_identifier(&["id".to_owned()], &[]);
        cm.set_primary_table(&php_array!["name" => full_table_name]);

        // The platform argument is unused upstream; the port has none.
        assert_eq!(
            cm.get_sequence_name().unwrap(),
            expected_sequence_name,
            "{name}"
        );
    }
}

#[test]
fn testIsIdentifierMappedSuperClass() {
    models::register();
    let class = ClassMetadataInfo::new(format!("{NS}DDC2700MappedSuperClass"), None, None);

    assert!(!class.is_identifier("foo"));
}

const ARRAY_OBJECT_ENTITY: &str = "Doctrine\\Tests\\ORM\\Mapping\\MyArrayObjectEntity";

#[test]
fn testCanInstantiateInternalPhpClassSubclass() {
    models::register();
    let class_metadata = ClassMetadataInfo::new(ARRAY_OBJECT_ENTITY, None, None);

    assert!(
        class_metadata
            .new_instance()
            .unwrap()
            .instance_of(ARRAY_OBJECT_ENTITY)
    );
}

#[test]
fn testCanInstantiateInternalPhpClassSubclassFromUnserializedMetadata() {
    models::register();
    let mut class_metadata = unserialize(ClassMetadataInfo::new(ARRAY_OBJECT_ENTITY, None, None));
    class_metadata
        .wakeup_reflection(&RuntimeReflectionService::new())
        .unwrap();

    assert!(
        class_metadata
            .new_instance()
            .unwrap()
            .instance_of(ARRAY_OBJECT_ENTITY)
    );
}

#[test]
fn testGetColumnNamesWithGivenFieldNames() {
    let mut metadata = metadata(CMS_USER);
    for (field, column) in [("status", "foo"), ("username", "bar"), ("name", "baz")] {
        metadata
            .map_field(php_array!["fieldName" => field, "type" => "string", "columnName" => column])
            .unwrap();
    }

    assert_eq!(
        metadata.get_column_names(Some(&["status".to_owned(), "name".to_owned()])),
        ["foo", "baz"]
    );
}

const TEST_ENTITY1: &str = "Doctrine\\Tests\\ORM\\Mapping\\TestEntity1";

#[test]
fn testInlineEmbeddable() {
    models::register();
    let mut class_metadata = ClassMetadataInfo::new(TEST_ENTITY1, None, None);
    class_metadata
        .map_embedded(
            php_array!["fieldName" => "test", "class" => TEST_ENTITY1, "columnPrefix" => false],
        )
        .unwrap();

    assert!(class_metadata.has_field("test"));
}

#[test]
fn testRejectsEmbeddableWithoutValidClassName() {
    models::register();
    let mut metadata = ClassMetadataInfo::new(TEST_ENTITY1, None, None);

    let error = expect_mapping_exception(metadata.map_embedded(
        php_array!["fieldName" => "embedded", "class" => "", "columnPrefix" => false],
    ));
    assert_eq!(
        text(&error),
        "The embed mapping 'embedded' misses the 'class' attribute."
    );
}

#[test]
fn testItAddingLifecycleCallbackOnEmbeddedClassIsIllegal() {
    let mut metadata = ClassMetadataInfo::new(TEST_CLASS, None, None);
    metadata.is_embedded_class = true;

    let error = expect_mapping_exception(metadata.add_lifecycle_callback("foo", "bar"));
    assert_eq!(
        text(&error),
        "Context: Attempt to register lifecycle callback \"foo\" on embedded class \"Doctrine\\Tests\\ORM\\Mapping\\ClassMetadataTest\".\nProblem: Registering lifecycle callbacks on embedded classes is not allowed."
    );
}

#[test]
fn testGettingAnFQCNForNullIsDeprecated() {
    use php2rust_doctrine_orm::deprecations;
    let metadata = ClassMetadataInfo::new(TEST_CLASS, None, None);

    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(cm::NULL_CLASS_NAME_DEPRECATION);
    let _ = metadata.fully_qualified_null_class_name();
    assert!(deprecations::count(cm::NULL_CLASS_NAME_DEPRECATION) > before);
}

#[test]
fn testItThrowsOnInvalidCallToGetAssociationMappedByTargetField() {
    let mut metadata = ClassMetadataInfo::new(TEST_CLASS, None, None);
    metadata
        .map_one_to_one(php_array!["fieldName" => "foo", "targetEntity" => "bar"])
        .unwrap();

    let error = metadata
        .get_association_mapped_by_target_field("foo")
        .unwrap_err();
    assert_eq!(error.class, "LogicException");
    assert_eq!(
        text(&error),
        "Context: Calling Doctrine\\ORM\\Mapping\\ClassMetadata::getAssociationMappedByTargetField() with \"foo\", which is the owning side of an association.\nProblem: The owning side of an association has no \"mappedBy\" field.\nSolution: Call Doctrine\\ORM\\Mapping\\ClassMetadata::isAssociationInverseSide() to check first."
    );
}

#[test]
fn testDiscriminatorMapWithSameClassMultipleTimesDeprecated() {
    use php2rust_doctrine_orm::deprecations;
    models::register();
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(cm::DUPLICATE_DISCRIMINATOR_DEPRECATION);

    let mut cm = ClassMetadataInfo::new(CMS_USER, None, None);
    cm.set_discriminator_map(&php_array!["foo" => CMS_USER, "bar" => CMS_USER])
        .unwrap();

    assert!(deprecations::count(cm::DUPLICATE_DISCRIMINATOR_DEPRECATION) > before);
}

fn map_id(cm: &mut ClassMetadataInfo, field: &str, position: Option<i64>) {
    let mut mapping = php_array!["fieldName" => field, "type" => "string", "id" => true];
    if let Some(position) = position {
        mapping.insert("idPosition", Value::Int(position));
    }
    cm.map_field(mapping).unwrap();
}

fn positions(cm: &ClassMetadataInfo) -> Vec<(&str, i64)> {
    cm.identifier_positions
        .iter()
        .map(|(k, v)| (k, *v))
        .collect()
}

#[test]
fn testCompositeIdentifierOrderingByPosition() {
    let mut cm = metadata(CMS_USER);
    map_id(&mut cm, "username", Some(2));
    map_id(&mut cm, "name", Some(1));

    assert_eq!(cm.identifier, ["name", "username"]);
}

#[test]
fn testCompositeIdentifierSamePositionPreservesInsertionOrder() {
    let mut cm = metadata(CMS_USER);
    map_id(&mut cm, "username", Some(1));
    map_id(&mut cm, "name", Some(1));

    assert_eq!(cm.identifier, ["username", "name"]);
}

#[test]
fn testCompositeIdentifierDefaultPositionPreservesInsertionOrder() {
    let mut cm = metadata(CMS_USER);
    map_id(&mut cm, "username", None);
    map_id(&mut cm, "name", None);

    assert_eq!(cm.identifier, ["username", "name"]);
}

#[test]
fn testCompositeIdentifierPositionSurvivesSerialization() {
    let mut cm = metadata(CMS_USER);
    map_id(&mut cm, "username", Some(2));
    map_id(&mut cm, "name", Some(1));

    let mut cm = unserialize(cm);
    cm.wakeup_reflection(&RuntimeReflectionService::new())
        .unwrap();

    assert_eq!(cm.identifier, ["name", "username"]);
    assert_eq!(positions(&cm), [("username", 2), ("name", 1)]);
}

#[test]
fn testSetIdentifierSortsByPosition() {
    let mut cm = metadata(CMS_USER);
    cm.set_identifier(
        &["username".to_owned(), "name".to_owned()],
        &[("username".to_owned(), 2), ("name".to_owned(), 1)],
    );

    assert_eq!(cm.identifier, ["name", "username"]);
    assert_eq!(positions(&cm), [("username", 2), ("name", 1)]);
}

#[test]
fn testSetIdentifierWithoutPositionsDefaultsToZero() {
    let mut cm = metadata(CMS_USER);
    cm.set_identifier(&["username".to_owned(), "name".to_owned()], &[]);

    assert_eq!(cm.identifier, ["username", "name"]);
    assert_eq!(positions(&cm), [("username", 0), ("name", 0)]);
}

#[test]
fn testClassNameMappingDiscriminatorValue() {
    models::register();
    let driver = php2rust_doctrine_orm::mapping::driver::xml_driver::XmlDriver::create(
        format!("{}/tests/data/Mapping/xml", env!("CARGO_MANIFEST_DIR")).as_str(),
        php2rust_doctrine_orm::mapping::driver::xml_driver::DEFAULT_FILE_EXTENSION,
        true,
    );
    let xml_element = driver
        .get_element("Doctrine\\Tests\\Models\\Customer\\CustomerType")
        .unwrap();
    assert_eq!(
        Some("Doctrine\\Tests\\Models\\Customer\\InternalCustomer"),
        xml_element
            .child("discriminator-map")
            .unwrap()
            .children_named("discriminator-mapping")[0]
            .attr("value")
    );
}
