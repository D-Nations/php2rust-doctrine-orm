//! Upstream: doctrine/orm tests/Tests/ORM/Tools/SchemaToolTest.php
//!
//! `getTestEntityManager()` is an entity manager on an in-memory `SQLite`
//! connection. The tables come from DBAL's table API; the index and
//! foreign key helpers read DBAL's editor API (`getIndexedColumns()`,
//! `getType()`, `getReferencedTableName()`), as upstream does when it
//! exists. A configuration change after the entity manager is made
//! (`setDefaultStringTypeSchemaLength()`, `setNamingStrategy()`) is made
//! before it; `setSchemaIgnoreClasses()` after a first schema is a second
//! entity manager. `assertSame()` of two indexes compares the references.
//! The listeners are `EventListener`s that downcast the event arguments.
//! The `StaticPHPDriver`'s call of a class's static `loadMetadata()` is
//! the class's `load_metadata()` fn.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_dbal::schema::{Index, IndexType, ReferentialAction, Schema, Table};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, ListenerError,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::ClassMetadataInfo;
use php2rust_doctrine_orm::mapping::class_metadata::INHERITANCE_TYPE_SINGLE_TABLE;
use php2rust_doctrine_orm::mapping::naming_strategy::CASE_LOWER;
use php2rust_doctrine_orm::mapping::{ClassMetadata, UnderscoreNamingStrategy};
use php2rust_doctrine_orm::tools::event::{GenerateSchemaEventArgs, GenerateSchemaTableEventArgs};
use php2rust_doctrine_orm::tools::schema_tool::SchemaTool;
use php2rust_doctrine_orm::tools::tool_events;
use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;

use orm::CMS_USER;

const CMS_ADDRESS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddress";
const CMS_ARTICLE: &str = "Doctrine\\Tests\\Models\\CMS\\CmsArticle";
const CMS_COMMENT: &str = "Doctrine\\Tests\\Models\\CMS\\CmsComment";
const CMS_EMPLOYEE: &str = "Doctrine\\Tests\\Models\\CMS\\CmsEmployee";
const CMS_GROUP: &str = "Doctrine\\Tests\\Models\\CMS\\CmsGroup";
const CMS_PHONENUMBER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber";
const COMPOSITE_ID_ENTITY: &str =
    "Doctrine\\Tests\\Models\\CompositeIdWithPosition\\CompositeIdEntity";
const JOINED_DERIVED_CHILD_CLASS: &str =
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedDerivedChildClass";
const JOINED_DERIVED_IDENTITY_CLASS: &str =
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedDerivedIdentityClass";
const JOINED_DERIVED_ROOT_CLASS: &str =
    "Doctrine\\Tests\\Models\\CompositeKeyInheritance\\JoinedDerivedRootClass";
const CARD: &str = "Doctrine\\Tests\\Models\\Enums\\Card";
const SUIT: &str = "Doctrine\\Tests\\Models\\Enums\\Suit";
const FORUM_AVATAR: &str = "Doctrine\\Tests\\Models\\Forum\\ForumAvatar";
const FORUM_USER: &str = "Doctrine\\Tests\\Models\\Forum\\ForumUser";
const GH10288_PEOPLE: &str = "Doctrine\\Tests\\Models\\GH10288\\GH10288People";
const NULL_DEFAULT_COLUMN: &str = "Doctrine\\Tests\\Models\\NullDefault\\NullDefaultColumn";

/// A class of this test file's namespace.
macro_rules! tools {
    ($name:literal) => {
        concat!("Doctrine\\Tests\\ORM\\Tools\\", $name)
    };
}

const FIRST_ENTITY: &str = tools!("FirstEntity");
const SECOND_ENTITY: &str = tools!("SecondEntity");

fn get_test_entity_manager() -> Arc<EntityManager> {
    orm::entity_manager(orm::connection(), None)
}

/// `$schemaTool->getSchemaFromMetadata()` of the classes' metadata.
fn schema_of(em: &Arc<EntityManager>, classes: &[&str]) -> Schema {
    let classes: Vec<_> = classes
        .iter()
        .map(|class| em.get_class_metadata(class).unwrap())
        .collect();
    SchemaTool::new(em)
        .get_schema_from_metadata(&classes)
        .unwrap()
}

/// A PHP array of string keys.
fn array(entries: &[(&str, Value)]) -> PhpArray {
    let mut array = PhpArray::new();
    for (key, value) in entries {
        array.insert(*key, value.clone());
    }
    array
}

fn get_indexed_columns(index: &Index) -> Vec<String> {
    index
        .get_indexed_columns()
        .unwrap()
        .iter()
        .map(|column| column.get_column_name().to_string())
        .collect()
}

fn column_is_indexed(table: &Table, column: &str) -> bool {
    table
        .get_indexes()
        .values()
        .any(|index| index.spans_columns(&[column.to_owned()]))
}

fn index_is_unique(index: &Index) -> bool {
    index.get_type().unwrap() == IndexType::Unique
}

fn index_is_fulltext(index: &Index) -> bool {
    index.get_type().unwrap() == IndexType::Fulltext
}

fn assert_index_is_unique(index: &Index) {
    assert!(index_is_unique(index), "Index should be unique.");
}

fn assert_index_is_fulltext(index: &Index) {
    assert!(index_is_fulltext(index), "Index should be fulltext.");
}

fn find_unique_non_primary_index(table: &Table) -> Option<&Index> {
    table
        .get_indexes()
        .iter()
        .filter(|(name, _)| name.to_lowercase() != "primary")
        .map(|(_, index)| index)
        .find(|index| index_is_unique(index))
}

fn get_index_predicate(index: &Index) -> Option<String> {
    index.get_predicate().unwrap()
}

fn names(names: &[php2rust_doctrine_dbal::schema::name::UnqualifiedName]) -> Vec<String> {
    names.iter().map(ToString::to_string).collect()
}

fn assert_table_has_single_foreign_key(
    table: &Table,
    local_columns: &[&str],
    foreign_table_name: &str,
    foreign_columns: &[&str],
    on_delete: Option<&str>,
) {
    let foreign_keys = table.get_foreign_keys();

    assert_eq!(1, foreign_keys.len());

    let foreign_key = foreign_keys.values().next().unwrap();

    assert_eq!(
        foreign_table_name,
        foreign_key.get_referenced_table_name().unwrap().to_string()
    );
    assert_eq!(
        local_columns,
        names(foreign_key.get_referencing_column_names().unwrap())
    );
    assert_eq!(
        foreign_columns,
        names(foreign_key.get_referenced_column_names().unwrap())
    );

    let on_delete_action = foreign_key.get_on_delete_action().unwrap();
    assert_eq!(
        on_delete,
        (on_delete_action != ReferentialAction::NoAction).then(|| on_delete_action.value())
    );
}

fn get_schema_table_for_entity(class_name: &str) -> Table {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[class_name]);
    let tables = schema.get_tables();

    assert_eq!(1, tables.len());

    tables[0].clone()
}

const CMS_CLASSES: [&str; 7] = [
    CMS_ADDRESS,
    CMS_ARTICLE,
    CMS_COMMENT,
    CMS_EMPLOYEE,
    CMS_GROUP,
    CMS_PHONENUMBER,
    CMS_USER,
];

#[test]
fn testAddUniqueIndexForUniqueFieldAttribute() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &CMS_CLASSES);

    assert!(
        schema.has_table("cms_users"),
        "Table cms_users should exist."
    );
    assert!(
        column_is_indexed(schema.get_table("cms_users").unwrap(), "username"),
        "username column should be indexed."
    );
}

#[test]
fn testAttributeOptionsArgument() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[tools!("TestEntityWithAttributeOptionsArgument")]);
    let table = schema
        .get_table("TestEntityWithAttributeOptionsArgument")
        .unwrap();

    for options in [
        table.get_options(),
        table.get_column("test").unwrap().get_platform_options(),
    ] {
        assert_eq!(Some(&Value::string("bar")), options.get(&"foo".into()));
        assert_eq!(
            Some(&Value::Array(array(&[("key", Value::string("val"))]))),
            options.get(&"baz".into())
        );
    }
}

#[test]
fn testPassColumnDefinitionToJoinColumn() {
    let custom_column_def = "MEDIUMINT(6) UNSIGNED NOT NULL";

    let em = get_test_entity_manager();

    let avatar = em.get_class_metadata(FORUM_AVATAR).unwrap();
    avatar
        .write()
        .field_mappings
        .get_mut("id")
        .unwrap()
        .column_definition = Some(custom_column_def.to_owned());

    let schema = schema_of(&em, &[FORUM_AVATAR, FORUM_USER]);

    assert!(schema.has_table("forum_users"));
    let table = schema.get_table("forum_users").unwrap();
    assert!(table.has_column("avatar_id"));
    assert_eq!(
        Some(custom_column_def),
        table
            .get_column("avatar_id")
            .unwrap()
            .get_column_definition()
    );
}

#[test]
fn testPassColumnOptionsToJoinColumn() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[tools!("GH6830Category"), tools!("GH6830Board")]);

    assert!(schema.has_table("GH6830Category"));
    assert!(schema.has_table("GH6830Board"));

    let table_category = schema.get_table("GH6830Category").unwrap();
    let table_board = schema.get_table("GH6830Board").unwrap();

    assert!(table_board.has_column("category_id"));

    assert_eq!(
        table_category.get_column("id").unwrap().get_fixed(),
        table_board.get_column("category_id").unwrap().get_fixed(),
        "Foreign key/join column should have the same value of option `fixed` as the referenced column",
    );

    assert_eq!(
        table_category
            .get_column("id")
            .unwrap()
            .get_platform_options(),
        table_board
            .get_column("category_id")
            .unwrap()
            .get_platform_options(),
        "Foreign key/join column should have the same custom options as the referenced column",
    );

    assert_eq!(
        &array(&[
            ("collation", Value::string("latin1_bin")),
            ("foo", Value::string("bar")),
        ]),
        table_board
            .get_column("category_id")
            .unwrap()
            .get_platform_options()
    );
}

/// `GenerateSchemaEventListener`.
#[derive(Default)]
struct GenerateSchemaEventListener {
    table_calls: AtomicUsize,
    schema_calls: AtomicUsize,
}

impl EventListener for GenerateSchemaEventListener {
    fn call(&self, event_name: &str, _event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        match event_name {
            tool_events::POST_GENERATE_SCHEMA_TABLE => {
                self.table_calls.fetch_add(1, Ordering::SeqCst);
            }
            tool_events::POST_GENERATE_SCHEMA => {
                self.schema_calls.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }
        Ok(())
    }
}

/// `$em->getEventManager()->addEventListener([postGenerateSchemaTable,
/// postGenerateSchema], $listener)`.
fn add_listener(em: &Arc<EntityManager>, events: &[&str], listener: Arc<dyn EventListener>) {
    em.with_event_manager(|evm| {
        evm.add_event_listener(events, listener);
    });
}

#[test]
fn testPostGenerateEvents() {
    let listener = Arc::new(GenerateSchemaEventListener::default());

    let em = get_test_entity_manager();
    add_listener(
        &em,
        &[
            tool_events::POST_GENERATE_SCHEMA_TABLE,
            tool_events::POST_GENERATE_SCHEMA,
        ],
        listener.clone(),
    );

    let _schema = schema_of(&em, &CMS_CLASSES);

    assert_eq!(
        CMS_CLASSES.len(),
        listener.table_calls.load(Ordering::SeqCst)
    );
    assert_eq!(1, listener.schema_calls.load(Ordering::SeqCst));
}

#[test]
fn testNullDefaultNotAddedToPlatformOptions() {
    let em = get_test_entity_manager();

    assert!(
        schema_of(&em, &[NULL_DEFAULT_COLUMN])
            .get_table("NullDefaultColumn")
            .unwrap()
            .get_column("nullDefault")
            .unwrap()
            .get_platform_options()
            .is_empty()
    );
}

#[test]
fn testEnumTypeAddedToCustomSchemaOptions() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[CARD]);
    let platform_options = schema
        .get_table("Card")
        .unwrap()
        .get_column("suit")
        .unwrap()
        .get_platform_options();

    assert_eq!(
        Some(&Value::string(SUIT)),
        platform_options.get(&"enumType".into())
    );
}

#[test]
fn testSchemaHasProperIndexesFromUniqueConstraintAttribute() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[tools!("UniqueConstraintAttributeModel")]);

    assert!(schema.has_table("unique_constraint_attribute_table"));
    let table = schema
        .get_table("unique_constraint_attribute_table")
        .unwrap();

    assert_eq!(2, table.get_indexes().len());
    assert!(table.has_index("primary"));
    assert!(table.has_index("uniq_hash"));
}

#[test]
fn testRemoveUniqueIndexOverruledByPrimaryKey() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[FIRST_ENTITY, SECOND_ENTITY]);

    assert!(
        schema.has_table("first_entity"),
        "Table first_entity should exist."
    );

    let table = schema.get_table("first_entity").unwrap();

    assert!(
        table.has_index("primary"),
        "Table should have a primary key."
    );

    let primary_key = table.get_index("primary").unwrap();
    let indexes = table.get_indexes();

    assert_eq!(1, indexes.len(), "there should be only one index");
    assert!(
        std::ptr::eq(primary_key, indexes.values().next().unwrap()),
        "index should be primary"
    );
}

/// `setInheritanceType(SINGLE_TABLE)` and `setDiscriminatorColumn()`.
fn set_discriminator_column(em: &Arc<EntityManager>, column: PhpArray) {
    let metadata = em.get_class_metadata(FIRST_ENTITY).unwrap();
    let mut metadata = metadata.write();
    metadata
        .set_inheritance_type(INHERITANCE_TYPE_SINGLE_TABLE)
        .unwrap();
    metadata.set_discriminator_column(Some(column)).unwrap();
}

#[test]
fn testSetDiscriminatorColumnWithoutLength() {
    let em = get_test_entity_manager();
    set_discriminator_column(
        &em,
        array(&[
            ("name", Value::string("discriminator")),
            ("type", Value::string("string")),
        ]),
    );

    let schema = schema_of(&em, &[FIRST_ENTITY]);

    assert!(schema.has_table("first_entity"));
    let table = schema.get_table("first_entity").unwrap();

    assert!(table.has_column("discriminator"));
    let column = table.get_column("discriminator").unwrap();

    assert_eq!(Some(255), column.get_length());
}

#[test]
fn testSetDefaultStringLength() {
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_default_string_type_schema_length(191);
    });
    let schema = schema_of(&em, &[FIRST_ENTITY]);

    assert!(schema.has_table("first_entity"));
    let table = schema.get_table("first_entity").unwrap();

    assert!(table.has_column("name"));
    let column = table.get_column("name").unwrap();

    assert_eq!(Some(191), column.get_length());
}

#[test]
fn testSetDefaultStringLengthDoesNotOverrideExplicitLength() {
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_default_string_type_schema_length(191);
    });
    let metadata = em.get_class_metadata(FIRST_ENTITY).unwrap();
    metadata
        .write()
        .field_mappings
        .get_mut("name")
        .unwrap()
        .length = Some(32);

    let schema = schema_of(&em, &[FIRST_ENTITY]);

    assert!(schema.has_table("first_entity"));
    let table = schema.get_table("first_entity").unwrap();

    assert!(table.has_column("name"));
    let column = table.get_column("name").unwrap();

    assert_eq!(Some(32), column.get_length());
}

#[test]
fn testSetDiscriminatorColumnWithCustomDefaultLength() {
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_default_string_type_schema_length(191);
    });
    set_discriminator_column(
        &em,
        array(&[
            ("name", Value::string("discriminator")),
            ("type", Value::string("string")),
        ]),
    );

    let schema = schema_of(&em, &[FIRST_ENTITY]);

    assert!(schema.has_table("first_entity"));
    let table = schema.get_table("first_entity").unwrap();

    assert!(table.has_column("discriminator"));
    let column = table.get_column("discriminator").unwrap();

    assert_eq!(Some(191), column.get_length());
}

#[test]
fn testSetDiscriminatorColumnWithEnumType() {
    let em = get_test_entity_manager();
    set_discriminator_column(
        &em,
        array(&[
            ("name", Value::string("discriminator")),
            ("type", Value::string("enum")),
            ("enumType", Value::string(GH10288_PEOPLE)),
        ]),
    );

    let schema = schema_of(&em, &[FIRST_ENTITY]);

    assert!(schema.has_table("first_entity"));
    let table = schema.get_table("first_entity").unwrap();

    assert!(table.has_column("discriminator"));
    let column = table.get_column("discriminator").unwrap();
    assert_eq!(
        Some(&Value::string(GH10288_PEOPLE)),
        column.get_platform_option("enumType")
    );
    assert_eq!(
        &PhpArray::list([Value::string("boss"), Value::string("employee")]),
        column.get_values()
    );

    let metadata = em.get_class_metadata(FIRST_ENTITY).unwrap();
    let e = metadata
        .write()
        .set_discriminator_map(&array(&[
            ("user", Value::string(CMS_USER)),
            ("employee", Value::string(CMS_EMPLOYEE)),
        ]))
        .unwrap_err();
    assert_eq!("Doctrine\\ORM\\Mapping\\MappingException", e.class);
    assert_eq!(
        format!(
            "The entries 'user' in the discriminator map of class '{FIRST_ENTITY}' do not correspond to enum cases of '{GH10288_PEOPLE}'."
        ),
        orm::message(&e)
    );
}

#[test]
fn testConflictingSingleTableInheritanceAssociationIndexesAreAddedOnce() {
    let em = get_test_entity_manager();
    let schema = schema_of(
        &em,
        &[
            tools!("GH12606Media"),
            tools!("GH12606Housing"),
            tools!("GH12606Location"),
            tools!("GH12606Person"),
        ],
    );

    let table = schema.get_table("gh12606_media").unwrap();

    assert!(column_is_indexed(table, "reference_id"));
}

#[test]
fn testConflictingSingleTableInheritanceAssociationsTargetingSameEntityStillGenerateForeignKey() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[tools!("GH12609Base"), tools!("GH12609Ref")]);

    // GH12609ChildOne and GH12609ChildThree both map a "ref_id" column to
    // GH12609Ref, but declare conflicting JoinColumn configuration (nullable:
    // false vs. nullable: true). There is no single foreign key definition that
    // could honor both associations at once, so - exactly as before the two-pass
    // rewrite introduced in GH-12528 - the association registered first
    // (GH12609ChildOne, which declares the column as NOT NULL) wins. What matters
    // for this regression test is that a foreign key is generated at all, instead
    // of being silently dropped as it was on 3.7.0.
    let table = schema.get_table("gh12609_base").unwrap();
    assert!(table.has_column("ref_id"));
    assert!(table.get_column("ref_id").unwrap().get_notnull());
    assert_eq!(1, table.get_foreign_keys().len());
}

#[test]
fn testIdenticalSingleTableInheritanceAssociationsShareOneForeignKey() {
    let em = get_test_entity_manager();
    let schema = schema_of(
        &em,
        &[
            tools!("GH12609IdenticalBase"),
            tools!("GH12609IdenticalRef"),
        ],
    );

    let table = schema.get_table("gh12609_identical_base").unwrap();

    assert!(table.has_column("ref_id"));
    assert_eq!(1, table.get_foreign_keys().len());
}

#[test]
fn testDerivedCompositeKey() {
    let em = get_test_entity_manager();
    let schema = schema_of(
        &em,
        &[
            JOINED_DERIVED_IDENTITY_CLASS,
            JOINED_DERIVED_ROOT_CLASS,
            JOINED_DERIVED_CHILD_CLASS,
        ],
    );

    assert!(schema.has_table("joined_derived_identity"));
    assert!(schema.has_table("joined_derived_root"));
    assert!(schema.has_table("joined_derived_child"));

    let root_table = schema.get_table("joined_derived_root").unwrap();
    let root_primary_key = root_table.get_primary_key_constraint().unwrap();
    assert!(root_primary_key.is_some());
    assert_eq!(
        vec!["keyPart1_id", "keyPart2"],
        names(root_primary_key.unwrap().get_column_names())
    );

    let child_table = schema.get_table("joined_derived_child").unwrap();
    let child_primary_key = child_table.get_primary_key_constraint().unwrap();
    assert!(child_primary_key.is_some());
    assert_eq!(
        vec!["keyPart1_id", "keyPart2"],
        names(child_primary_key.unwrap().get_column_names())
    );

    let child_table_foreign_keys = child_table.get_foreign_keys();

    assert_eq!(2, child_table_foreign_keys.len());

    let expected_columns: [(&str, Columns); 2] = [
        ("joined_derived_identity", (&["keyPart1_id"], &["id"])),
        (
            "joined_derived_root",
            (&["keyPart1_id", "keyPart2"], &["keyPart1_id", "keyPart2"]),
        ),
    ];

    for foreign_key in child_table_foreign_keys.values() {
        let referenced = foreign_key.get_referenced_table_name().unwrap().to_string();
        let Some((_, (local_columns, foreign_columns))) = expected_columns
            .iter()
            .find(|(name, _)| *name == referenced)
        else {
            panic!("{referenced}");
        };

        assert_eq!(
            *local_columns,
            names(foreign_key.get_referencing_column_names().unwrap())
        );
        assert_eq!(
            *foreign_columns,
            names(foreign_key.get_referenced_column_names().unwrap())
        );
    }
}

/// The local and foreign columns of a foreign key.
type Columns = (&'static [&'static str], &'static [&'static str]);

/// A row of `provideJoinedHierarchiesWithAssociationAsIdentifier()`: the
/// row name, the class names, the root table name, its `onDelete` and the
/// child table names.
type JoinedHierarchy = (
    &'static str,
    Vec<&'static str>,
    &'static str,
    Option<&'static str>,
    Vec<&'static str>,
);

/// `provideJoinedHierarchiesWithAssociationAsIdentifier()`.
fn provide_joined_hierarchies_with_association_as_identifier() -> [JoinedHierarchy; 4] {
    [
        (
            "one-to-one identifier",
            vec![
                tools!("GH12651Target"),
                tools!("GH12651Root"),
                tools!("GH12651Child"),
            ],
            "gh12651_root",
            Some("CASCADE"),
            vec!["gh12651_child"],
        ),
        (
            "many-to-one identifier",
            vec![
                tools!("GH12651Target"),
                tools!("GH12651ManyToOneRoot"),
                tools!("GH12651ManyToOneChild"),
            ],
            "gh12651_many_to_one_root",
            Some("CASCADE"),
            vec!["gh12651_many_to_one_child"],
        ),
        (
            "identifier without onDelete",
            vec![
                tools!("GH12651Target"),
                tools!("GH12651NoOnDeleteRoot"),
                tools!("GH12651NoOnDeleteChild"),
            ],
            "gh12651_no_on_delete_root",
            None,
            vec!["gh12651_no_on_delete_child"],
        ),
        // Like any other JOINED child table, the leaf table references the root table, not its parent table.
        (
            "three-level hierarchy",
            vec![
                tools!("GH12651Target"),
                tools!("GH12651ThreeLevelRoot"),
                tools!("GH12651ThreeLevelMiddle"),
                tools!("GH12651ThreeLevelLeaf"),
            ],
            "gh12651_three_level_root",
            Some("CASCADE"),
            vec!["gh12651_three_level_middle", "gh12651_three_level_leaf"],
        ),
    ]
}

#[test]
fn testJoinedChildTableForeignKeyReferencesRootTableWhenIdentifierIsAnAssociation() {
    for (row, class_names, root_table_name, root_table_on_delete, child_table_names) in
        provide_joined_hierarchies_with_association_as_identifier()
    {
        // The inherited identifier association and the FK to the root table share the same
        // local columns, and thus the same generated name. Only one of them must be added.
        let em = get_test_entity_manager();
        let schema = schema_of(&em, &class_names);

        assert_table_has_single_foreign_key(
            schema.get_table(root_table_name).unwrap(),
            &["target_id"],
            "gh12651_target",
            &["id"],
            root_table_on_delete,
        );

        for child_table_name in child_table_names {
            // JoinedSubclassPersister::delete() only deletes the root row and relies on this
            // ON DELETE CASCADE constraint to delete the rows of the child tables.
            assert_table_has_single_foreign_key(
                schema.get_table(child_table_name).unwrap(),
                &["target_id"],
                root_table_name,
                &["target_id"],
                Some("CASCADE"),
            );
        }
        let _ = row;
    }
}

#[test]
fn testIndexesBasedOnFields() {
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_naming_strategy(Arc::new(UnderscoreNamingStrategy::new(CASE_LOWER)));
    });

    let schema = schema_of(&em, &[tools!("IndexByFieldEntity")]);
    let table = schema.get_table("field_index").unwrap();

    assert_eq!(
        vec!["index", "field_name"],
        get_indexed_columns(table.get_index("index").unwrap())
    );
    assert_eq!(
        vec!["index", "table"],
        get_indexed_columns(table.get_index("uniq").unwrap())
    );
}

/// The fields of `IncorrectIndexByFieldEntity::loadMetadata()` and
/// `IncorrectUniqueConstraintByFieldEntity::loadMetadata()`.
fn map_incorrect_fields(metadata: &mut ClassMetadataInfo) {
    metadata
        .map_field(php_array! { "id" => true, "fieldName" => "id" })
        .unwrap();

    metadata
        .map_field(php_array! { "fieldName" => "index" })
        .unwrap();

    metadata
        .map_field(php_array! { "fieldName" => "table" })
        .unwrap();

    metadata
        .map_field(php_array! { "fieldName" => "fieldName" })
        .unwrap();
}

/// `{columns: ['index'], fields: ['fieldName']}`.
fn incorrect_index() -> Value {
    Value::Array(php_array! {
        "columns" => Value::Array(PhpArray::list([Value::string("index")])),
        "fields" => Value::Array(PhpArray::list([Value::string("fieldName")])),
    })
}

/// `IncorrectIndexByFieldEntity::loadMetadata()`.
fn incorrect_index_by_field_entity_load_metadata(metadata: &mut ClassMetadataInfo) {
    map_incorrect_fields(metadata);

    metadata.set_primary_table(&php_array! {
        "indexes" => Value::Array(PhpArray::list([incorrect_index()])),
    });
}

/// `IncorrectUniqueConstraintByFieldEntity::loadMetadata()`.
fn incorrect_unique_constraint_by_field_entity_load_metadata(metadata: &mut ClassMetadataInfo) {
    map_incorrect_fields(metadata);

    metadata.set_primary_table(&php_array! {
        "uniqueConstraints" => Value::Array(PhpArray::list([incorrect_index()])),
    });
}

/// `new ClassMetadata($className)`, `initializeReflection()` and the
/// `StaticPHPDriver`'s `loadMetadataForClass()`, then
/// `getSchemaFromMetadata()`: a `MappingException`.
fn assert_incorrect_class_throws(class_name: &str, load_metadata: fn(&mut ClassMetadataInfo)) {
    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_naming_strategy(Arc::new(UnderscoreNamingStrategy::new(CASE_LOWER)));
    });

    let schema_tool = SchemaTool::new(&em);
    let class = Arc::new(ClassMetadata::new(class_name, None, None));

    class
        .write()
        .initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    load_metadata(&mut class.write());

    let e = schema_tool.get_schema_from_metadata(&[class]).unwrap_err();
    assert_eq!("Doctrine\\ORM\\Mapping\\MappingException", e.class);
}

#[test]
fn testIncorrectIndexesBasedOnFields() {
    assert_incorrect_class_throws(
        tools!("IncorrectIndexByFieldEntity"),
        incorrect_index_by_field_entity_load_metadata,
    );
}

#[test]
fn testIncorrectUniqueConstraintsBasedOnFields() {
    assert_incorrect_class_throws(
        tools!("IncorrectUniqueConstraintByFieldEntity"),
        incorrect_unique_constraint_by_field_entity_load_metadata,
    );
}

#[test]
fn testConfigurationSchemaIgnoredEntity() {
    let em = get_test_entity_manager();
    let classes = [FIRST_ENTITY, SECOND_ENTITY];

    let schema = schema_of(&em, &classes);

    assert!(
        schema.has_table("first_entity"),
        "Table first_entity should exist."
    );
    assert!(
        schema.has_table("second_entity"),
        "Table second_entity should exist."
    );

    let em = orm::entity_manager_with(orm::connection(), |config| {
        config.set_schema_ignore_classes(vec![SECOND_ENTITY.to_owned()]);
    });

    let schema = schema_of(&em, &classes);

    assert!(
        schema.has_table("first_entity"),
        "Table first_entity should exist."
    );
    assert!(
        !schema.has_table("second_entity"),
        "Table second_entity should not exist."
    );
}

#[test]
fn testLoadUniqueConstraintWithoutName() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[tools!("GH11314Entity")]);

    assert!(schema.has_table("GH11314Entity"));

    let table_entity = schema.get_table("GH11314Entity").unwrap();

    assert!(table_entity.has_index("uniq_2d81a3ed5bf54558875f7fd5"));

    let table_index = table_entity
        .get_index("uniq_2d81a3ed5bf54558875f7fd5")
        .unwrap();

    assert_eq!(IndexType::Unique, table_index.get_type().unwrap());

    assert_eq!(
        vec!["field", "anotherField"],
        get_indexed_columns(table_index)
    );
}

#[test]
fn testQuotedIdentifiers() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[tools!("QuotedEntity")]);

    assert!(
        schema.has_table("quoted-table"),
        "Table quoted-table should exist."
    );

    let table = schema.get_table("quoted-table").unwrap();

    assert!(table.has_index("IDX_AA2790FB50D14D90"));

    let object_name = table.get_object_name().unwrap();
    assert!(object_name.get_unqualified_name().is_quoted());
    assert_eq!(
        "quoted-table",
        object_name.get_unqualified_name().get_value()
    );

    let primary_key = table.get_primary_key_constraint().unwrap();
    assert!(primary_key.is_some());
    let primary_key = primary_key.unwrap();
    assert_eq!(1, primary_key.get_column_names().len());

    let pk_column = &primary_key.get_column_names()[0];
    assert!(pk_column.get_identifier().is_quoted());
    assert_eq!("quoted-id", pk_column.get_identifier().get_value());
}

#[test]
fn testCompositeIdPositionRespectedInSchema() {
    let em = get_test_entity_manager();
    let schema = schema_of(&em, &[COMPOSITE_ID_ENTITY]);

    let table = schema.get_table("CompositeIdEntity").unwrap();

    let pk_columns = names(
        table
            .get_primary_key_constraint()
            .unwrap()
            .unwrap()
            .get_column_names(),
    );

    assert_eq!(vec!["first", "second", "third"], pk_columns);
}

/// `new Schema()`.
fn new_schema() -> Schema {
    Schema::new(Vec::new(), Vec::new(), None, &[]).unwrap()
}

/// The anonymous listener of `testOverwritingSchemaInListener`.
struct SchemaOverwritingListener;

impl EventListener for SchemaOverwritingListener {
    fn call(&self, _event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        let any: &dyn Any = event_args;
        if let Some(event_args) = any.downcast_ref::<GenerateSchemaTableEventArgs>() {
            event_args.set_schema(new_schema());
        }
        if let Some(event_args) = any.downcast_ref::<GenerateSchemaEventArgs>() {
            event_args.set_schema(new_schema());
        }
        Ok(())
    }
}

#[test]
fn testOverwritingSchemaInListener() {
    let em = get_test_entity_manager();

    add_listener(
        &em,
        &[
            tool_events::POST_GENERATE_SCHEMA_TABLE,
            tool_events::POST_GENERATE_SCHEMA,
        ],
        Arc::new(SchemaOverwritingListener),
    );

    let schema = schema_of(&em, &[CMS_ADDRESS]);

    assert!(
        !schema.has_table("cms_address"),
        "The original schema should have been overwritten by the listener, so cms_address table should not exist.",
    );
}

#[test]
fn testFulltextIndexIsPreservedAlongsideUniqueConstraint() {
    let table = get_schema_table_for_entity(tools!("GH5593FulltextWithUniqueConstraint"));

    assert!(table.has_index("search_idx"));
    assert!(table.has_index("uniq_title"));

    let fulltext_index = table.get_index("search_idx").unwrap();
    let unique_index = table.get_index("uniq_title").unwrap();

    assert_eq!(vec!["title"], get_indexed_columns(fulltext_index));
    assert_eq!(vec!["title"], get_indexed_columns(unique_index));
    assert_index_is_fulltext(fulltext_index);
    assert_index_is_unique(unique_index);
    assert!(!index_is_unique(fulltext_index));
}

#[test]
fn testFulltextIndexIsPreservedAlongsideColumnLevelUnique() {
    let table = get_schema_table_for_entity(tools!("GH5593FulltextWithColumnUnique"));

    assert!(table.has_index("search_idx"));
    assert_index_is_fulltext(table.get_index("search_idx").unwrap());
    assert_eq!(
        vec!["title"],
        get_indexed_columns(table.get_index("search_idx").unwrap())
    );

    let unique_index = find_unique_non_primary_index(&table);
    assert!(
        unique_index.is_some(),
        "Column-level unique mapping should produce a unique index."
    );
    let unique_index = unique_index.unwrap();
    assert_index_is_unique(unique_index);
    assert_eq!(vec!["title"], get_indexed_columns(unique_index));
    assert!(!index_is_unique(table.get_index("search_idx").unwrap()));
}

#[test]
fn testCompositeFulltextAndUniqueIndexesOnSameColumnsArePreserved() {
    let table = get_schema_table_for_entity(tools!("GH5593CompositeFulltextAndUnique"));

    assert!(table.has_index("fulltext_ab"));
    assert!(table.has_index("fulltext_c"));
    assert!(table.has_index("uniq_ab"));
    assert!(table.has_index("uniq_ac"));

    let index = |name: &str| table.get_index(name).unwrap();

    assert_eq!(vec!["a", "b"], get_indexed_columns(index("fulltext_ab")));
    assert_eq!(vec!["c"], get_indexed_columns(index("fulltext_c")));
    assert_eq!(vec!["a", "b"], get_indexed_columns(index("uniq_ab")));
    assert_eq!(vec!["a", "c"], get_indexed_columns(index("uniq_ac")));

    assert_index_is_fulltext(index("fulltext_ab"));
    assert_index_is_fulltext(index("fulltext_c"));
    assert_index_is_unique(index("uniq_ab"));
    assert_index_is_unique(index("uniq_ac"));
}

#[test]
fn testUnflaggedIndexCoveredByUniqueConstraintIsDeduplicated() {
    let table = get_schema_table_for_entity(tools!("GH5593UnflaggedIndexWithUniqueConstraint"));

    assert!(!table.has_index("idx_name"));
    assert!(table.has_index("uniq_name"));
    assert!(table.has_index("idx_other"));

    let index = |name: &str| table.get_index(name).unwrap();

    assert_index_is_unique(index("uniq_name"));
    assert_eq!(vec!["name"], get_indexed_columns(index("uniq_name")));
    assert_eq!(vec!["other"], get_indexed_columns(index("idx_other")));
    assert!(!index_is_unique(index("idx_other")));
    assert!(!index_is_fulltext(index("idx_other")));
}

#[test]
fn testPartialIndexOptionsStillPreventDeduplication() {
    let table = get_schema_table_for_entity(tools!("GH5593PartialIndexWithUniqueConstraint"));

    assert!(table.has_index("idx_partial"));
    assert!(!table.has_index("idx_covered_partial"));
    assert!(table.has_index("uniq_name"));
    assert!(table.has_index("uniq_code"));

    let index = |name: &str| table.get_index(name).unwrap();

    assert_eq!(vec!["name"], get_indexed_columns(index("idx_partial")));
    assert_eq!(vec!["name"], get_indexed_columns(index("uniq_name")));
    assert_eq!(vec!["code"], get_indexed_columns(index("uniq_code")));
    assert_eq!(
        Some("name IS NOT NULL".to_owned()),
        get_index_predicate(index("idx_partial"))
    );
    assert_eq!(
        Some("code IS NOT NULL".to_owned()),
        get_index_predicate(index("uniq_code"))
    );
    assert_eq!(None, get_index_predicate(index("uniq_name")));
}

/// The anonymous listener of `testOverwritingTableInListener`.
struct TableOverwritingListener;

impl EventListener for TableOverwritingListener {
    fn call(&self, _event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        let any: &dyn Any = event_args;
        let event_args = any
            .downcast_ref::<GenerateSchemaTableEventArgs>()
            .expect("GenerateSchemaTableEventArgs");
        event_args.set_class_table(Table::new("just_address").unwrap());
        Ok(())
    }
}

#[test]
fn testOverwritingTableInListener() {
    let em = get_test_entity_manager();

    add_listener(
        &em,
        &[tool_events::POST_GENERATE_SCHEMA_TABLE],
        Arc::new(TableOverwritingListener),
    );

    let schema = schema_of(&em, &[CMS_ADDRESS]);
    assert!(
        schema.has_table("just_address"),
        "The original table should have been overwritten by the listener, so just_address table should exist.",
    );
    assert!(
        !schema.has_table("cms_address"),
        "The original table should have been overwritten by the listener, so cms_address table should not exist.",
    );
}
