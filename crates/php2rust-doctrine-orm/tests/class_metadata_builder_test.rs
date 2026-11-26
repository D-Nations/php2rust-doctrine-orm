//! Port of `ClassMetadataBuilderTest`. `assertIsFluent()` compares the
//! returned builder by address; `assertEquals` on the mapping objects
//! compares their class and, loosely, their `toArray()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ClassMetadataBuilderTest.php

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
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::mapping::builder::{ClassMetadataBuilder, ColumnPrefixArgument};
use php2rust_doctrine_orm::mapping::class_metadata::{self as cm, ClassMetadata, sort_direction};
use php2rust_doctrine_orm::mapping::ordered_map::OrderedMap;
use php2rust_doctrine_orm::mapping::{
    AssociationClass, AssociationKind, AssociationMapping, DiscriminatorColumnMapping,
    EmbeddedClassMapping, FieldMapping, mapping_exception,
};
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
const CMS_GROUP: &str = "Doctrine\\Tests\\Models\\CMS\\CmsGroup";
const NAME: &str = "Doctrine\\Tests\\Models\\ValueObjects\\Name";
const STRING_ORDER_BY_DEPRECATION: &str = "https://github.com/doctrine/orm/issues/11313";

fn set_up() -> (Arc<ClassMetadata>, ClassMetadataBuilder) {
    models::register();
    let cm = Arc::new(ClassMetadata::new(CMS_USER, None, None));
    cm.write()
        .initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    let builder = ClassMetadataBuilder::new(Arc::clone(&cm));
    (cm, builder)
}

/// `assertIsFluent()`.
fn assert_is_fluent(builder: &ClassMetadataBuilder, ret: &ClassMetadataBuilder) {
    assert!(
        std::ptr::eq(builder, ret),
        "Return Value has to be same instance as used builder"
    );
}

fn expect_mapping_exception<T>(result: Result<T, Thrown>) {
    let Err(error) = result else {
        panic!("expected a MappingException");
    };
    assert!(thrown_is_a(&error, mapping_exception::CLASS), "{error:?}");
}

fn assert_equals(expected: &Value, actual: &Value) {
    assert!(
        phpunit_equals(expected, actual),
        "{expected:?} != {actual:?}"
    );
}

fn embedded(column_prefix: Value) -> EmbeddedClassMapping {
    EmbeddedClassMapping::from_mapping_array(&php_array! {
        "class" => NAME,
        "columnPrefix" => column_prefix,
        "declaredField" => Value::Null,
        "originalField" => Value::Null,
    })
    .unwrap()
}

fn association(kind: AssociationKind, mapping: PhpArray) -> AssociationMapping {
    AssociationMapping::from_mapping_array(AssociationClass::Final(kind), &mapping).unwrap()
}

/// `assertEquals(['name' => $mapping], $this->cm->associationMappings)`.
fn assert_associations(
    expected: &[(&str, AssociationMapping)],
    actual: &OrderedMap<AssociationMapping>,
) {
    assert_eq!(
        expected.iter().map(|(name, _)| *name).collect::<Vec<_>>(),
        actual.keys().collect::<Vec<_>>()
    );
    for (name, expected) in expected {
        let actual = actual.get(name).unwrap();
        assert_eq!(expected.class, actual.class, "{name}");
        assert_equals(
            &Value::Array(expected.to_array().unwrap()),
            &Value::Array(actual.to_array().unwrap()),
        );
    }
}

#[test]
fn testSetMappedSuperClass() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_mapped_super_class());
    assert!(cm.read().is_mapped_superclass);
    assert!(!cm.read().is_embedded_class);
}

#[test]
fn testSetEmbedable() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_embeddable());
    assert!(cm.read().is_embedded_class);
    assert!(!cm.read().is_mapped_superclass);
}

#[test]
fn testAddEmbeddedWithOnlyRequiredParams() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.add_embedded("name", NAME, None).unwrap());

    let cm = cm.read();
    assert_eq!(vec!["name"], cm.embedded_classes.keys().collect::<Vec<_>>());
    assert_eq!(
        &embedded(Value::Null),
        cm.embedded_classes.get("name").unwrap()
    );
}

#[test]
fn testAddEmbeddedWithPrefix() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .add_embedded("name", NAME, Some(ColumnPrefixArgument::Prefix("nm_")))
            .unwrap(),
    );

    let cm = cm.read();
    assert_eq!(vec!["name"], cm.embedded_classes.keys().collect::<Vec<_>>());
    assert_eq!(
        &embedded(Value::from("nm_")),
        cm.embedded_classes.get("name").unwrap()
    );
}

#[test]
fn testCreateEmbeddedWithoutExtraParams() {
    let (cm, builder) = set_up();
    let embedded_builder = builder.create_embedded("name", NAME);

    assert!(cm.read().embedded_classes.get("name").is_none());

    assert_is_fluent(&builder, embedded_builder.build().unwrap());
    assert_eq!(
        &embedded(Value::Null),
        cm.read().embedded_classes.get("name").unwrap()
    );
}

#[test]
fn testCreateEmbeddedWithColumnPrefix() {
    let (cm, builder) = set_up();
    // setColumnPrefix() returns the builder itself (by value here).
    let embedded_builder = builder
        .create_embedded("name", NAME)
        .set_column_prefix("nm_");

    assert_is_fluent(&builder, embedded_builder.build().unwrap());

    assert_eq!(
        &embedded(Value::from("nm_")),
        cm.read().embedded_classes.get("name").unwrap()
    );
}

#[test]
fn testSetCustomRepositoryClass() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_custom_repository_class(CMS_GROUP));
    assert_eq!(
        Some(CMS_GROUP),
        cm.read().custom_repository_class_name.as_deref()
    );
}

#[test]
fn testSetReadOnly() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_read_only());
    assert!(cm.read().is_read_only);
}

fn table(cm: &ClassMetadata, key: &str) -> Value {
    cm.read()
        .table
        .get(&key.into())
        .cloned()
        .unwrap_or(Value::Null)
}

#[test]
fn testSetTable() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_table("users"));
    assert_eq!(Value::from("users"), table(&cm, "name"));
}

fn users_idx() -> Value {
    Value::Array(
        php_array! { "users_idx" => php_array! { "columns" => php_array!["username", "name"] } },
    )
}

#[test]
fn testAddIndex() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder.add_index(&["username", "name"], "users_idx"),
    );
    assert_equals(&users_idx(), &table(&cm, "indexes"));
}

#[test]
fn testAddUniqueConstraint() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder.add_unique_constraint(&["username", "name"], "users_idx"),
    );
    assert_equals(&users_idx(), &table(&cm, "uniqueConstraints"));
}

#[test]
fn testSetPrimaryTableRelated() {
    let (cm, builder) = set_up();
    builder.add_unique_constraint(&["username", "name"], "users_idx");
    builder.add_index(&["username", "name"], "users_idx");
    builder.set_table("users");

    assert_equals(
        &Value::Array(php_array! {
            "name" => "users",
            "indexes" => users_idx(),
            "uniqueConstraints" => users_idx(),
        }),
        &Value::Array(cm.read().table.clone()),
    );
}

#[test]
fn testSetInheritanceJoined() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_joined_table_inheritance().unwrap());
    assert_eq!(cm::INHERITANCE_TYPE_JOINED, cm.read().inheritance_type);
}

#[test]
fn testSetInheritanceSingleTable() {
    let (cm, builder) = set_up();
    assert_is_fluent(&builder, builder.set_single_table_inheritance().unwrap());
    assert_eq!(
        cm::INHERITANCE_TYPE_SINGLE_TABLE,
        cm.read().inheritance_type
    );
}

#[test]
fn testSetDiscriminatorColumn() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .set_discriminator_column("discr", "string", 124, None, None, PhpArray::new())
            .unwrap(),
    );
    assert_eq!(
        Some(
            DiscriminatorColumnMapping::from_mapping_array(&php_array! {
                "fieldName" => "discr",
                "name" => "discr",
                "type" => "string",
                "length" => 124,
                "columnDefinition" => Value::Null,
                "enumType" => Value::Null,
                "options" => PhpArray::new(),
            })
            .unwrap()
        ),
        cm.read().discriminator_column
    );
}

#[test]
fn testAddDiscriminatorMapClass() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .add_discriminator_map_class("test", CMS_USER)
            .unwrap(),
    );
    assert_is_fluent(
        &builder,
        builder
            .add_discriminator_map_class("test2", CMS_GROUP)
            .unwrap(),
    );

    assert_equals(
        &Value::Array(php_array! { "test" => CMS_USER, "test2" => CMS_GROUP }),
        &Value::Array(cm.read().discriminator_map.clone()),
    );
    assert_equals(&Value::from("test"), &cm.read().discriminator_value);
}

#[test]
fn testChangeTrackingPolicyExplicit() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder.set_change_tracking_policy_deferred_explicit(),
    );
    assert_eq!(
        cm::CHANGETRACKING_DEFERRED_EXPLICIT,
        cm.read().change_tracking_policy
    );
}

#[test]
fn testAddField() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .add_field("name", "string", PhpArray::new())
            .unwrap(),
    );
    let cm = cm.read();
    let mapping = cm.get_field_mapping("name").unwrap();
    assert_eq!("name", mapping.field_name);
    assert_eq!("name", mapping.column_name);
    assert_eq!("string", mapping.r#type);
}

#[test]
fn testCreateField() {
    let (cm, builder) = set_up();
    let field_builder = builder.create_field("name", "string");

    assert!(cm.read().field_mappings.get("name").is_none());
    assert_is_fluent(&builder, field_builder.build().unwrap());
    let cm = cm.read();
    let mapping = cm.get_field_mapping("name").unwrap();
    assert_eq!("name", mapping.field_name);
    assert_eq!("name", mapping.column_name);
    assert_eq!("string", mapping.r#type);
}

#[test]
fn testCreateVersionedField() {
    let (cm, builder) = set_up();
    builder
        .create_field("name", "integer")
        .column_name("username")
        .length(124)
        .nullable(true)
        .column_definition("foobar")
        .unique(true)
        .is_version_field()
        .build()
        .unwrap();
    assert_eq!(
        &FieldMapping::from_mapping_array(&php_array! {
            "columnDefinition" => "foobar",
            "columnName" => "username",
            "options" => php_array! { "default" => 1 },
            "fieldName" => "name",
            "length" => 124,
            "type" => "integer",
            "nullable" => true,
            "unique" => true,
        })
        .unwrap(),
        cm.read().field_mappings.get("name").unwrap()
    );
}

#[test]
fn testCreatePrimaryField() {
    let (cm, builder) = set_up();
    builder
        .create_field("id", "integer")
        .make_primary_key()
        .generated_value("AUTO")
        .build()
        .unwrap();

    assert_eq!(vec!["id"], cm.read().identifier);
    assert_eq!(
        &FieldMapping::from_mapping_array(
            &php_array! { "columnName" => "id", "fieldName" => "id", "id" => true, "type" => "integer" },
        )
        .unwrap(),
        cm.read().field_mappings.get("id").unwrap()
    );
}

#[test]
fn testCreateUnsignedOptionField() {
    let (cm, builder) = set_up();
    builder
        .create_field("state", "integer")
        .option("unsigned", Value::Bool(true))
        .build()
        .unwrap();

    assert_eq!(
        &FieldMapping::from_mapping_array(&php_array! {
            "fieldName" => "state",
            "type" => "integer",
            "options" => php_array! { "unsigned" => true },
            "columnName" => "state",
        })
        .unwrap(),
        cm.read().field_mappings.get("state").unwrap()
    );
}

#[test]
fn testAddLifecycleEvent() {
    let (cm, builder) = set_up();
    builder
        .add_lifecycle_event("getStatus", "postLoad")
        .unwrap();

    let cm = cm.read();
    assert_eq!(
        vec![("postLoad", &vec!["getStatus".to_owned()])],
        cm.lifecycle_callbacks.iter().collect::<Vec<_>>()
    );
}

/// The cascade `cascadeAll()` gives.
fn cascade_all() -> PhpArray {
    php_array! { 0 => "remove", 1 => "persist", 2 => "refresh", 3 => "detach" }
}

fn group_join_column(nullable: bool, unique: bool) -> PhpArray {
    php_array! {
        0 => php_array! {
            "name" => "group_id",
            "referencedColumnName" => "id",
            "nullable" => nullable,
            "unique" => unique,
            "onDelete" => "CASCADE",
            "columnDefinition" => Value::Null,
        },
    }
}

fn to_one_owning(r#type: i64, join_columns: PhpArray, cascade: PhpArray, fetch: i64) -> PhpArray {
    php_array! {
        "fieldName" => "groups",
        "targetEntity" => CMS_GROUP,
        "cascade" => cascade,
        "fetch" => fetch,
        "joinColumns" => join_columns,
        "type" => r#type,
        "inversedBy" => Value::Null,
        "isOwningSide" => true,
        "sourceEntity" => CMS_USER,
        "sourceToTargetKeyColumns" => php_array! { "group_id" => "id" },
        "joinColumnFieldNames" => php_array! { "group_id" => "group_id" },
        "targetToSourceKeyColumns" => php_array! { "id" => "group_id" },
        "orphanRemoval" => false,
    }
}

#[test]
fn testCreateManyToOne() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_many_to_one("groups", CMS_GROUP)
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .cascade_all()
            .fetch_extra_lazy()
            .build()
            .unwrap(),
    );

    assert_associations(
        &[(
            "groups",
            association(
                AssociationKind::ManyToOne,
                to_one_owning(2, group_join_column(true, false), cascade_all(), 4),
            ),
        )],
        &cm.read().association_mappings,
    );
}

#[test]
fn testCreateManyToOneWithIdentity() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_many_to_one("groups", CMS_GROUP)
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .cascade_all()
            .fetch_extra_lazy()
            .make_primary_key()
            .build()
            .unwrap(),
    );

    let mut expected = to_one_owning(2, group_join_column(false, false), cascade_all(), 4);
    expected.insert("id", Value::Bool(true));
    assert_associations(
        &[("groups", association(AssociationKind::ManyToOne, expected))],
        &cm.read().association_mappings,
    );
}

#[test]
fn testCreateOneToOne() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_one_to_one("groups", CMS_GROUP)
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .cascade_all()
            .fetch_extra_lazy()
            .build()
            .unwrap(),
    );

    assert_associations(
        &[(
            "groups",
            association(
                AssociationKind::OneToOneOwningSide,
                to_one_owning(1, group_join_column(true, true), cascade_all(), 4),
            ),
        )],
        &cm.read().association_mappings,
    );
}

#[test]
fn testCreateOneToOneWithIdentity() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_one_to_one("groups", CMS_GROUP)
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .cascade_all()
            .fetch_extra_lazy()
            .make_primary_key()
            .build()
            .unwrap(),
    );

    let mut expected = to_one_owning(1, group_join_column(false, false), cascade_all(), 4);
    expected.insert("id", Value::Bool(true));
    assert_associations(
        &[(
            "groups",
            association(AssociationKind::OneToOneOwningSide, expected),
        )],
        &cm.read().association_mappings,
    );
}

#[test]
fn testThrowsExceptionOnCreateOneToOneWithIdentityOnInverseSide() {
    let (_cm, builder) = set_up();
    expect_mapping_exception(
        builder
            .create_one_to_one("groups", CMS_GROUP)
            .mapped_by("test")
            .fetch_extra_lazy()
            .make_primary_key()
            .build(),
    );
}

#[test]
fn testCreateManyToMany() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_many_to_many("groups", CMS_GROUP)
            .set_join_table("groups_users")
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .add_inverse_join_column("user_id", "id", true, false, None, None)
            .cascade_all()
            .fetch_extra_lazy()
            .build()
            .unwrap(),
    );

    assert_associations(
        &[(
            "groups",
            association(
                AssociationKind::ManyToManyOwningSide,
                php_array! {
                    "fieldName" => "groups",
                    "targetEntity" => CMS_GROUP,
                    "cascade" => cascade_all(),
                    "fetch" => 4,
                    "joinTable" => php_array! {
                        "joinColumns" => php_array! {
                            0 => php_array! {
                                "name" => "group_id",
                                "referencedColumnName" => "id",
                                "unique" => false,
                                "onDelete" => "CASCADE",
                                "columnDefinition" => Value::Null,
                            },
                        },
                        "inverseJoinColumns" => php_array! {
                            0 => php_array! {
                                "name" => "user_id",
                                "referencedColumnName" => "id",
                                "unique" => false,
                                "onDelete" => Value::Null,
                                "columnDefinition" => Value::Null,
                            },
                        },
                        "name" => "groups_users",
                    },
                    "type" => 8,
                    "inversedBy" => Value::Null,
                    "isOwningSide" => true,
                    "isOnDeleteCascade" => true,
                    "sourceEntity" => CMS_USER,
                    "relationToSourceKeyColumns" => php_array! { "group_id" => "id" },
                    "joinTableColumns" => php_array! { 0 => "group_id", 1 => "user_id" },
                    "relationToTargetKeyColumns" => php_array! { "user_id" => "id" },
                    "orphanRemoval" => false,
                },
            ),
        )],
        &cm.read().association_mappings,
    );
}

#[test]
fn testThrowsExceptionOnCreateManyToManyWithIdentity() {
    let (_cm, builder) = set_up();
    expect_mapping_exception(
        builder
            .create_many_to_many("groups", CMS_GROUP)
            .make_primary_key()
            .set_join_table("groups_users")
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .add_inverse_join_column("user_id", "id", true, false, None, None)
            .cascade_all()
            .fetch_extra_lazy()
            .build(),
    );
}

#[test]
fn testCreateOneToMany() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_one_to_many("groups", CMS_GROUP)
            .mapped_by("test")
            .set_order_by(php_array! { "test" => sort_direction(SortDirection::Ascending) })
            .set_index_by("test")
            .build()
            .unwrap(),
    );

    assert_associations(
        &[(
            "groups",
            association(
                AssociationKind::OneToMany,
                php_array! {
                    "fieldName" => "groups",
                    "targetEntity" => CMS_GROUP,
                    "mappedBy" => "test",
                    "orderBy" => php_array! { "test" => sort_direction(SortDirection::Ascending) },
                    "indexBy" => "test",
                    "type" => 4,
                    "isOwningSide" => false,
                    "sourceEntity" => CMS_USER,
                    "fetch" => 2,
                    "cascade" => PhpArray::new(),
                    "orphanRemoval" => false,
                },
            ),
        )],
        &cm.read().association_mappings,
    );
}

fn order_by_test(cm: &ClassMetadata) -> Value {
    cm.read()
        .association_mappings
        .get("groups")
        .unwrap()
        .order_by
        .get(&"test".into())
        .cloned()
        .unwrap()
}

#[test]
fn testCreateOneToManyWithLegacySortDirection() {
    let (cm, builder) = set_up();
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(STRING_ORDER_BY_DEPRECATION);
    builder
        .create_one_to_many("groups", CMS_GROUP)
        .mapped_by("test")
        .set_order_by(php_array! { "test" => "ASC" })
        .build()
        .unwrap();
    assert!(deprecations::count(STRING_ORDER_BY_DEPRECATION) > before);

    assert_equals(
        &sort_direction(SortDirection::Ascending),
        &order_by_test(&cm),
    );
}

#[test]
fn testCreateManyToManyWithLegacySortDirection() {
    let (cm, builder) = set_up();
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(STRING_ORDER_BY_DEPRECATION);
    builder
        .create_many_to_many("groups", CMS_GROUP)
        .set_order_by(php_array! { "test" => "ASC" })
        .build()
        .unwrap();
    assert!(deprecations::count(STRING_ORDER_BY_DEPRECATION) > before);

    assert_equals(
        &sort_direction(SortDirection::Ascending),
        &order_by_test(&cm),
    );
}

#[test]
fn testThrowsExceptionOnCreateOneToManyWithIdentity() {
    let (_cm, builder) = set_up();
    expect_mapping_exception(
        builder
            .create_one_to_many("groups", CMS_GROUP)
            .make_primary_key()
            .mapped_by("test")
            .set_order_by(php_array! { "test" => sort_direction(SortDirection::Ascending) })
            .set_index_by("test")
            .build(),
    );
}

#[test]
fn testOrphanRemovalOnCreateOneToOne() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_one_to_one("groups", CMS_GROUP)
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .orphan_removal()
            .build()
            .unwrap(),
    );

    let mut expected = to_one_owning(1, group_join_column(true, true), PhpArray::new(), 2);
    expected.insert("orphanRemoval", Value::Bool(true));
    assert_associations(
        &[(
            "groups",
            association(AssociationKind::OneToOneOwningSide, expected),
        )],
        &cm.read().association_mappings,
    );
}

#[test]
fn testOrphanRemovalOnCreateOneToMany() {
    let (cm, builder) = set_up();
    assert_is_fluent(
        &builder,
        builder
            .create_one_to_many("groups", CMS_GROUP)
            .mapped_by("test")
            .orphan_removal()
            .build()
            .unwrap(),
    );

    assert_associations(
        &[(
            "groups",
            association(
                AssociationKind::OneToMany,
                php_array! {
                    "fieldName" => "groups",
                    "targetEntity" => CMS_GROUP,
                    "mappedBy" => "test",
                    "type" => 4,
                    "isOwningSide" => false,
                    "sourceEntity" => CMS_USER,
                    "fetch" => 2,
                    "cascade" => PhpArray::new(),
                    "orphanRemoval" => true,
                },
            ),
        )],
        &cm.read().association_mappings,
    );
}

#[test]
fn testExceptionOnOrphanRemovalOnManyToOne() {
    let (_cm, builder) = set_up();
    expect_mapping_exception(
        builder
            .create_many_to_one("groups", CMS_GROUP)
            .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
            .orphan_removal()
            .build(),
    );
}

#[test]
fn testOrphanRemovalOnManyToMany() {
    let (cm, builder) = set_up();
    builder
        .create_many_to_many("groups", CMS_GROUP)
        .add_join_column("group_id", "id", true, false, Some("CASCADE"), None)
        .orphan_removal()
        .build()
        .unwrap();

    assert_associations(
        &[(
            "groups",
            association(
                AssociationKind::ManyToManyOwningSide,
                php_array! {
                    "fieldName" => "groups",
                    "targetEntity" => CMS_GROUP,
                    "cascade" => PhpArray::new(),
                    "fetch" => 2,
                    "joinTable" => php_array! {
                        "joinColumns" => php_array! {
                            0 => php_array! {
                                "name" => "group_id",
                                "referencedColumnName" => "id",
                                "unique" => false,
                                "onDelete" => "CASCADE",
                                "columnDefinition" => Value::Null,
                            },
                        },
                        "inverseJoinColumns" => php_array! {
                            0 => php_array! {
                                "name" => "cmsgroup_id",
                                "referencedColumnName" => "id",
                                "onDelete" => "CASCADE",
                            },
                        },
                        "name" => "cmsuser_cmsgroup",
                    },
                    "type" => 8,
                    "inversedBy" => Value::Null,
                    "isOwningSide" => true,
                    "sourceEntity" => CMS_USER,
                    "isOnDeleteCascade" => true,
                    "relationToSourceKeyColumns" => php_array! { "group_id" => "id" },
                    "joinTableColumns" => php_array! { 0 => "group_id", 1 => "cmsgroup_id" },
                    "relationToTargetKeyColumns" => php_array! { "cmsgroup_id" => "id" },
                    "orphanRemoval" => true,
                },
            ),
        )],
        &cm.read().association_mappings,
    );
}
