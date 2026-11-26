//! Port of `ClassMetadataFactoryTest`. The entity manager is not ported:
//! `createEntityManager()` is a `FactoryConfiguration` with the driver,
//! and the event listeners are its hooks. PHPUnit's platform stubs are
//! `TestPlatform` (tests/common/platform.rs).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/ClassMetadataFactoryTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;
#[path = "common/platform.rs"]
mod platform;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use php_value::{Thrown, Value, php_array};
use php2rust_doctrine_orm::id::IdGenerator;
use php2rust_doctrine_orm::mapping::class_metadata::{self as cm, ClassMetadata};
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::{
    ClassMetadataFactory, FactoryConfiguration, mapping_exception,
};
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::mapping::{
    AbstractClassMetadataFactory, ClassMetadata as PersistenceClassMetadata,
    ClassMetadataFactoryState, ReflectionService, RuntimeReflectionService,
};
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;
use platform::TestPlatform;

const NS: &str = "Doctrine\\Tests\\ORM\\Mapping\\";
const INVALID_CUSTOM_GENERATOR: &str = "Doctrine\\ORM\\Mapping\\Exception\\InvalidCustomGenerator";
const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
const CMS_ARTICLE: &str = "Doctrine\\Tests\\Models\\CMS\\CmsArticle";

fn ns(class: &str) -> String {
    format!("{NS}{class}")
}

fn text(error: &Thrown) -> String {
    String::from_utf8_lossy(error.message.as_bytes()).into_owned()
}

/// `Doctrine\Tests\Mocks\MetadataDriverMock`.
struct MetadataDriverMock;

impl MappingDriver for MetadataDriverMock {
    fn load_metadata_for_class(
        &self,
        _class_name: &str,
        _metadata: &dyn PersistenceClassMetadata,
    ) -> Result<(), Thrown> {
        Ok(())
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(Vec::new())
    }

    fn is_transient(&self, _class_name: &str) -> Result<bool, Thrown> {
        Ok(false)
    }
}

/// `createEntityManager()`: the configuration, on a platform that
/// supports identity columns.
fn create_entity_manager(driver: Arc<dyn MappingDriver>) -> FactoryConfiguration {
    models::register();
    FactoryConfiguration::new(driver)
}

fn attribute_driver() -> Arc<dyn MappingDriver> {
    Arc::new(AttributeDriver::new(Vec::<String>::new(), true).unwrap())
}

fn factory(driver: Arc<dyn MappingDriver>) -> ClassMetadataFactory {
    let cmf = ClassMetadataFactory::new();
    cmf.set_entity_manager(create_entity_manager(driver));
    cmf
}

/// `ClassMetadataFactoryTestSubject`: `newClassMetadataInstance()` returns
/// the metadata set for the class.
struct ClassMetadataFactoryTestSubject {
    inner: ClassMetadataFactory,
    state: ClassMetadataFactoryState<ClassMetadata>,
    mock_metadata: Mutex<HashMap<String, Arc<ClassMetadata>>>,
}

impl ClassMetadataFactoryTestSubject {
    fn new() -> Self {
        Self {
            inner: ClassMetadataFactory::new(),
            state: ClassMetadataFactoryState::new(),
            mock_metadata: Mutex::new(HashMap::new()),
        }
    }

    fn set_metadata_for_class(&self, class_name: &str, metadata: Arc<ClassMetadata>) {
        self.mock_metadata
            .lock()
            .unwrap()
            .insert(class_name.to_owned(), metadata);
    }
}

impl AbstractClassMetadataFactory for ClassMetadataFactoryTestSubject {
    type Metadata = ClassMetadata;

    fn factory_state(&self) -> &ClassMetadataFactoryState<ClassMetadata> {
        &self.state
    }

    fn initialize(&self) -> Result<(), Thrown> {
        self.inner.initialize()?;
        self.state.set_initialized(true);
        Ok(())
    }

    fn get_driver(&self) -> Result<Arc<dyn MappingDriver>, Thrown> {
        self.inner.get_driver()
    }

    fn wakeup_reflection(
        &self,
        class: &Arc<ClassMetadata>,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        self.inner.wakeup_reflection(class, refl_service)
    }

    fn initialize_reflection(
        &self,
        class: &Arc<ClassMetadata>,
        refl_service: &dyn ReflectionService,
    ) -> Result<(), Thrown> {
        self.inner.initialize_reflection(class, refl_service)
    }

    fn is_entity(&self, class: &ClassMetadata) -> bool {
        self.inner.is_entity(class)
    }

    fn do_load_metadata(
        &self,
        class: &Arc<ClassMetadata>,
        parent: Option<&Arc<ClassMetadata>>,
        root_entity_found: bool,
        non_superclass_parents: &[String],
    ) -> Result<(), Thrown> {
        self.inner
            .do_load_metadata(class, parent, root_entity_found, non_superclass_parents)
    }

    fn new_class_metadata_instance(&self, class_name: &str) -> Result<Arc<ClassMetadata>, Thrown> {
        self.mock_metadata
            .lock()
            .unwrap()
            .get(class_name)
            .cloned()
            .ok_or_else(|| {
                Thrown::new(
                    "InvalidArgumentException",
                    format!("No mock metadata found for class {class_name}."),
                    0,
                )
            })
    }

    fn on_not_found_metadata(
        &self,
        class_name: &str,
    ) -> Result<Option<Arc<ClassMetadata>>, Thrown> {
        self.inner.on_not_found_metadata(class_name)
    }
}

fn create_test_factory() -> ClassMetadataFactoryTestSubject {
    let cmf = ClassMetadataFactoryTestSubject::new();
    cmf.inner
        .set_entity_manager(create_entity_manager(Arc::new(MetadataDriverMock)));
    cmf
}

/// `setUpCmfForPlatform()`.
fn set_up_cmf_for_platform(
    platform: TestPlatform,
    preferences: Vec<(String, i64)>,
) -> ClassMetadataFactoryTestSubject {
    let cmf = ClassMetadataFactoryTestSubject::new();
    let mut configuration = create_entity_manager(Arc::new(MetadataDriverMock));
    configuration.platform = Arc::new(platform);
    configuration.identity_generation_preferences = preferences;
    cmf.inner.set_entity_manager(configuration);
    cmf
}

/// `createValidClassMetadata()`.
fn create_valid_class_metadata() -> Arc<ClassMetadata> {
    models::register();
    // Self-made metadata
    let cm1 = ClassMetadata::new(ns("TestEntity1"), None, None);
    {
        let mut cm1 = cm1.write();
        cm1.initialize_reflection(&RuntimeReflectionService::new())
            .unwrap();
        cm1.set_primary_table(&php_array! { "name" => "`group`" });
        // Add a mapped field
        cm1.map_field(php_array! { "fieldName" => "name", "type" => "string" })
            .unwrap();
        // Add a mapped field
        cm1.map_field(php_array! { "fieldName" => "id", "type" => "integer", "id" => true })
            .unwrap();
        // and a mapped association
        cm1.map_one_to_one(
            php_array! { "fieldName" => "other", "targetEntity" => "TestEntity1", "mappedBy" => "this" },
        )
        .unwrap();
        // and an association on the owning side
        let join_columns =
            php_array![php_array! { "name" => "other_id", "referencedColumnName" => "id" },];
        cm1.map_one_to_one(php_array! {
            "fieldName" => "association",
            "targetEntity" => "TestEntity1",
            "joinColumns" => join_columns,
        })
        .unwrap();
        // and an id generator type
        cm1.set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    }
    Arc::new(cm1)
}

fn table_name(class: &ClassMetadata) -> Option<Value> {
    class.read().table.get(&"name".into()).cloned()
}

#[test]
fn testGetMetadataForSingleClass() {
    let cm1 = create_valid_class_metadata();

    // SUT
    let cmf = ClassMetadataFactory::new();
    let mut configuration = create_entity_manager(Arc::new(MetadataDriverMock));
    // A platform that supports sequences.
    configuration.platform = Arc::new(TestPlatform::new(platform::POSTGRESQL));
    cmf.set_entity_manager(configuration);
    let name = cm1.read().name.clone();
    cmf.set_metadata_for(&name, Arc::clone(&cm1));

    // Prechecks
    {
        let info = cm1.read();
        assert!(info.parent_classes.is_empty());
        assert_eq!(cm::INHERITANCE_TYPE_NONE, info.inheritance_type);
        assert!(info.has_field("name"));
        assert_eq!(2, info.association_mappings.len());
        assert_eq!(cm::GENERATOR_TYPE_AUTO, info.generator_type);
    }
    assert_eq!(Some(Value::from("group")), table_name(&cm1));

    // Go
    let cm_map1 = cmf.get_metadata_for(&name).unwrap();

    assert!(Arc::ptr_eq(&cm1, &cm_map1));
    assert_eq!(Some(Value::from("group")), table_name(&cm_map1));
    assert_eq!(
        Some(&Value::Bool(true)),
        cm_map1.read().table.get(&"quoted".into())
    );
    assert!(cm_map1.read().parent_classes.is_empty());
    assert!(cm_map1.read().has_field("name"));
}

#[test]
fn testGetMetadataForReturnsLoadedCustomIdGenerator() {
    let cm1 = create_valid_class_metadata();
    {
        let mut info = cm1.write();
        info.set_id_generator_type(cm::GENERATOR_TYPE_CUSTOM);
        info.custom_generator_definition = Some(php_array! { "class" => ns("CustomIdGenerator") });
    }
    let cmf = create_test_factory();
    let name = cm1.read().name.clone();
    cmf.set_metadata_for_class(&name, cm1);

    let actual = cmf.get_metadata_for(&name).unwrap();

    assert_eq!(cm::GENERATOR_TYPE_CUSTOM, actual.read().generator_type);
    assert_eq!(
        Some(IdGenerator::Custom {
            class: ns("CustomIdGenerator")
        }),
        actual.read().id_generator
    );
}

#[test]
fn testPostgresSwitchesToIdentityColumnsWhenDbal4IsUsed() {
    let cm = create_valid_class_metadata();
    cm.write().set_id_generator_type(cm::GENERATOR_TYPE_AUTO);
    let cmf = set_up_cmf_for_platform(TestPlatform::new(platform::POSTGRESQL), Vec::new());
    let name = cm.read().name.clone();
    cmf.set_metadata_for_class(&name, cm);

    let metadata = cmf.get_metadata_for(&name).unwrap();

    assert_eq!(cm::GENERATOR_TYPE_IDENTITY, metadata.read().generator_type);
}

#[test]
fn testGetMetadataForThrowsExceptionOnUnknownCustomGeneratorClass() {
    let cm1 = create_valid_class_metadata();
    {
        let mut info = cm1.write();
        info.set_id_generator_type(cm::GENERATOR_TYPE_CUSTOM);
        info.custom_generator_definition = Some(php_array! { "class" => "NotExistingGenerator" });
    }
    let cmf = create_test_factory();
    let name = cm1.read().name.clone();
    cmf.set_metadata_for_class(&name, cm1);

    // InvalidCustomGenerator is an ORMException.
    let error = cmf.get_metadata_for(&name).unwrap_err();
    assert_eq!(INVALID_CUSTOM_GENERATOR, error.class);
}

#[test]
fn testGetMetadataForThrowsExceptionOnMissingCustomGeneratorDefinition() {
    let cm1 = create_valid_class_metadata();
    cm1.write().set_id_generator_type(cm::GENERATOR_TYPE_CUSTOM);
    let cmf = create_test_factory();
    let name = cm1.read().name.clone();
    cmf.set_metadata_for_class(&name, cm1);

    // InvalidCustomGenerator is an ORMException.
    let error = cmf.get_metadata_for(&name).unwrap_err();
    assert_eq!(INVALID_CUSTOM_GENERATOR, error.class);
}

/// `createMock(MappingDriver::class)` with `isTransient()` expected twice.
struct TransientDriverMock {
    calls: Mutex<usize>,
}

impl MappingDriver for TransientDriverMock {
    fn load_metadata_for_class(
        &self,
        _class_name: &str,
        _metadata: &dyn PersistenceClassMetadata,
    ) -> Result<(), Thrown> {
        Ok(())
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(Vec::new())
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        *self.calls.lock().unwrap() += 1;
        Ok(class_name == CMS_USER)
    }
}

#[test]
fn testIsTransient() {
    let driver = Arc::new(TransientDriverMock {
        calls: Mutex::new(0),
    });
    let cmf = factory(Arc::clone(&driver) as Arc<dyn MappingDriver>);

    assert!(cmf.is_transient(CMS_USER).unwrap());
    assert!(!cmf.is_transient(CMS_ARTICLE).unwrap());
    assert_eq!(2, *driver.calls.lock().unwrap());
}

#[test]
fn testGetAllMetadataWorksWithBadConnection() {
    // DDC-3551: the configuration holds the platform rather than a
    // connection that could fail to give one; getAllMetadata() does not
    // read it.
    let cmf = factory(Arc::new(MetadataDriverMock));

    // getting all the metadata should work, even if get DatabasePlatform blows up
    let metadata = cmf.get_all_metadata().unwrap();
    // this will just be an empty array - there was no error
    assert!(metadata.is_empty());
}

#[test]
#[allow(clippy::too_many_lines, reason = "the upstream test, assertion by assertion")]
fn testQuoteMetadata() {
    const QUOTE: &str = "Doctrine\\Tests\\Models\\Quote\\";
    let cmf = factory(attribute_driver());

    let user_metadata = cmf.get_metadata_for(&format!("{QUOTE}User")).unwrap();
    let phone_metadata = cmf.get_metadata_for(&format!("{QUOTE}Phone")).unwrap();
    let group_metadata = cmf.get_metadata_for(&format!("{QUOTE}Group")).unwrap();
    let address_metadata = cmf.get_metadata_for(&format!("{QUOTE}Address")).unwrap();
    let user_metadata = user_metadata.read();
    let phone_metadata = phone_metadata.read();
    let group_metadata = group_metadata.read();
    let address_metadata = address_metadata.read();

    // Phone Class Metadata
    let number = phone_metadata.field_mappings.get("number").unwrap();
    assert_eq!(Some(true), number.quoted);
    assert_eq!("phone-number", number.column_name);

    let user = phone_metadata.association_mappings.get("user").unwrap();
    assert_eq!(Some(true), user.join_columns[0].quoted);
    assert_eq!("user-id", user.join_columns[0].name);
    assert_eq!("user-id", user.join_columns[0].referenced_column_name);

    // User Group Metadata
    assert_eq!(
        Some(true),
        group_metadata.field_mappings.get("id").unwrap().quoted
    );
    assert_eq!(
        Some(true),
        group_metadata.field_mappings.get("name").unwrap().quoted
    );

    assert_eq!(
        "user-id",
        user_metadata.field_mappings.get("id").unwrap().column_name
    );
    assert_eq!(
        "user-name",
        user_metadata
            .field_mappings
            .get("name")
            .unwrap()
            .column_name
    );

    let user = group_metadata.association_mappings.get("parent").unwrap();
    assert_eq!(Some(true), user.join_columns[0].quoted);
    assert_eq!("parent-id", user.join_columns[0].name);
    assert_eq!("group-id", user.join_columns[0].referenced_column_name);

    // Address Class Metadata
    assert_eq!(
        Some(true),
        address_metadata.field_mappings.get("id").unwrap().quoted
    );
    assert_eq!(
        Some(true),
        address_metadata.field_mappings.get("zip").unwrap().quoted
    );

    assert_eq!(
        "address-id",
        address_metadata
            .field_mappings
            .get("id")
            .unwrap()
            .column_name
    );
    assert_eq!(
        "address-zip",
        address_metadata
            .field_mappings
            .get("zip")
            .unwrap()
            .column_name
    );

    let user = address_metadata.association_mappings.get("user").unwrap();
    assert_eq!(Some(true), user.join_columns[0].quoted);
    assert_eq!("user-id", user.join_columns[0].name);
    assert_eq!("user-id", user.join_columns[0].referenced_column_name);

    // User Class Metadata
    assert_eq!(
        Some(true),
        user_metadata.field_mappings.get("id").unwrap().quoted
    );
    assert_eq!(
        Some(true),
        user_metadata.field_mappings.get("name").unwrap().quoted
    );

    assert_eq!(
        "user-id",
        user_metadata.field_mappings.get("id").unwrap().column_name
    );
    assert_eq!(
        "user-name",
        user_metadata
            .field_mappings
            .get("name")
            .unwrap()
            .column_name
    );

    let groups = user_metadata.association_mappings.get("groups").unwrap();
    let join_table = groups.join_table.as_ref().unwrap();
    assert_eq!(Some(true), join_table.quoted);
    assert_eq!(Some(true), join_table.join_columns[0].quoted);
    assert_eq!("quote-users-groups", join_table.name);
    assert_eq!("user-id", join_table.join_columns[0].name);
    assert_eq!("user-id", join_table.join_columns[0].referenced_column_name);

    assert_eq!(Some(true), join_table.inverse_join_columns[0].quoted);
    assert_eq!("group-id", join_table.inverse_join_columns[0].name);
    assert_eq!(
        "group-id",
        join_table.inverse_join_columns[0].referenced_column_name
    );
}

#[test]
fn testFallbackLoadingCausesEventTriggeringThatCanModifyFetchedMetadata() {
    let metadata = Arc::new(ClassMetadata::new("Foo", None, None));
    let mut configuration = create_entity_manager(Arc::new(MetadataDriverMock));
    let found = Arc::clone(&metadata);
    configuration
        .on_class_metadata_not_found
        .push(Arc::new(move |class_name: &str| {
            // getFoundMetadata() is null: this is the only listener.
            assert_eq!("Foo", class_name);
            Ok(Some(Arc::clone(&found)))
        }));
    let cmf = ClassMetadataFactory::new();
    cmf.set_entity_manager(configuration);

    assert!(Arc::ptr_eq(
        &metadata,
        &cmf.get_metadata_for("Foo").unwrap()
    ));
}

#[test]
fn testInheritsIdGeneratorMappingFromEmbeddable() {
    let cmf = factory(attribute_driver());

    let user_metadata = cmf
        .get_metadata_for("Doctrine\\Tests\\Models\\DDC4006\\DDC4006User")
        .unwrap();

    assert!(user_metadata.read().is_id_generator_identity());
}

#[test]
fn testIdentifierPositionsAreInheritedFromMappedSuperclass() {
    let cmf = factory(attribute_driver());

    let metadata = cmf
        .get_metadata_for("Doctrine\\Tests\\Models\\CompositeIdWithPosition\\CompositeIdEntity")
        .unwrap();
    let metadata = metadata.read();

    assert_eq!(vec!["first", "second", "third"], metadata.identifier);
    assert_eq!(
        vec![("first", 1), ("second", 2), ("third", 3)],
        metadata
            .identifier_positions
            .iter()
            .map(|(name, position)| (name, *position))
            .collect::<Vec<_>>()
    );
}

#[test]
fn testInvalidSubClassCase() {
    let cmf = factory(attribute_driver());

    let error = cmf.get_metadata_for(&ns("Shape")).unwrap_err();

    assert!(thrown_is_a(&error, mapping_exception::CLASS), "{error:?}");
    assert_eq!(
        "Entity class 'Doctrine\\Tests\\ORM\\Mapping\\cube' used in the discriminator map of class 'Doctrine\\Tests\\ORM\\Mapping\\Shape' does not exist.",
        text(&error)
    );
}
