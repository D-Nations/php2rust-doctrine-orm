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
use php2rust_doctrine_orm::mapping::naming_strategy::{CASE_UPPER, UnderscoreNamingStrategy};
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

fn assert_array_equals(expected: PhpArray, actual: &PhpArray) {
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
    assert!(cm.sub_classes.is_empty());
    assert!(cm.parent_classes.is_empty());
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
        php_array!["USER_ID" => "ID"],
        &one.source_to_target_key_columns,
    );
    assert_array_equals(
        php_array!["USER_ID" => "USER_ID"],
        &one.join_column_field_names,
    );
    assert_array_equals(
        php_array!["ID" => "USER_ID"],
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
        php_array!["CMS_ADDRESS_ID" => "ID"],
        &many.relation_to_source_key_columns,
    );
    assert_array_equals(
        php_array!["CMS_USER_ID" => "ID"],
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
