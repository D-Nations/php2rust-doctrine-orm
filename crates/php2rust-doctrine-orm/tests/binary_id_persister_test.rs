//! Port of `BinaryIdPersisterTest`, on `SQLite`. `BinaryIdType` and
//! `BinaryId` are Rust types (`BinaryId::generate()` takes counter-based bytes,
//! not `random_bytes()`); the class locator of `Models/BinaryPrimaryKey` lists
//! its entity, `Category`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Persisters/BinaryIdPersisterTest.php
//! Upstream: doctrine/orm tests/Tests/Models/BinaryPrimaryKey/BinaryId.php tests/Tests/Models/BinaryPrimaryKey/BinaryIdType.php

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
#[path = "common/ticket.rs"]
mod ticket;

use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use php_value::{Object, PhpArray, PhpObject, PhpString, Value, php_array};
use php2rust_doctrine_dbal::ParameterType;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata::FETCH_EAGER;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_orm::tools::schema_validator::SchemaValidator;
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;

use orm::{append, list, new_object, s, set};

const CATEGORY: &str = "Doctrine\\Tests\\Models\\BinaryPrimaryKey\\Category";
const BINARY_ID: &str = "Doctrine\\Tests\\Models\\BinaryPrimaryKey\\BinaryId";
const BINARY_ID_TYPE: &str = "Doctrine\\Tests\\Models\\BinaryPrimaryKey\\BinaryIdType";
/// `BinaryIdType::NAME`.
const BINARY_ID_TYPE_NAME: &str = "binary_id";
/// `BinaryId::LENGTH`.
const LENGTH: usize = 6;

/// `BinaryId`: its `hexId`, and `__toString()` is `getBytes()`.
#[derive(Debug)]
struct BinaryId {
    hex_id: String,
}

impl BinaryId {
    /// `BinaryId::new()`.
    fn generate() -> Value {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
        Self::from_bytes(&n[n.len() - LENGTH..])
    }

    /// `BinaryId::fromBytes($value)`.
    fn from_bytes(value: &[u8]) -> Value {
        let mut hex_id = String::new();
        for byte in value {
            write!(hex_id, "{byte:02x}").unwrap();
        }
        let object: Object = Arc::new(Self { hex_id });
        Value::Object(object)
    }

    /// `getBytes()`.
    fn get_bytes(&self) -> Vec<u8> {
        (0..self.hex_id.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&self.hex_id[i..i + 2], 16).unwrap())
            .collect()
    }
}

impl PhpObject for BinaryId {
    fn class(&self) -> &str {
        BINARY_ID
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("hexId", s(&self.hex_id));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(PhpString::from(self.get_bytes()))
    }
}

/// `BinaryIdType extends Type`.
#[derive(Debug)]
struct BinaryIdType;

impl Type for BinaryIdType {
    fn class(&self) -> &'static str {
        BINARY_ID_TYPE
    }

    fn get_sql_declaration(
        &self,
        _column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_binary_type_declaration_sql(&php_array! {
            "length" => i64::try_from(LENGTH).unwrap(),
            "fixed" => true,
        })
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        match value {
            Value::Null => Ok(Value::Null),
            Value::Object(object) if object.class() == BINARY_ID => {
                Ok(Value::String(object.to_php_string().unwrap()))
            }
            other => panic!("Unexpected value: {other:?}"),
        }
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        match value {
            Value::Null => Ok(Value::Null),
            Value::Object(object) if object.class() == BINARY_ID => Ok(value.clone()),
            other => Ok(BinaryId::from_bytes(
                php_value::ops::to_string(other).unwrap().as_bytes(),
            )),
        }
    }

    fn get_binding_type(&self) -> ParameterType {
        ParameterType::Binary
    }
}

/// `new Category($name, $parent)`.
fn category(name: &str, parent: Option<&Object>) -> Object {
    let category = new_object(CATEGORY);
    set(&category, "id", BinaryId::generate());
    set(&category, "name", s(name));
    set(
        &category,
        "parent",
        parent.map_or(Value::Null, |p| Value::Object(p.clone())),
    );
    set(
        &category,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    if let Some(parent) = parent {
        // $parent?->addChild($this)
        append(parent, "children", Value::Object(category.clone()));
    }
    category
}

/// `FileClassLocator::createFromDirectories([__DIR__ . '/../../Models/BinaryPrimaryKey'])`:
/// its entity.
#[derive(Debug)]
struct ModelsLocator;

impl ClassLocator for ModelsLocator {
    fn get_class_names(&self) -> Result<Vec<String>, php_value::Thrown> {
        Ok(vec![CATEGORY.to_owned()])
    }
}

/// `createEntityManager()`.
async fn create_entity_manager() -> Arc<EntityManager> {
    if !php2rust_doctrine_dbal::types::has_type(BINARY_ID_TYPE_NAME) {
        ticket::add_type(BINARY_ID_TYPE_NAME, Arc::new(BinaryIdType));
    }
    let entity_manager = orm::entity_manager_with(orm::connection(), |config| {
        let locator: Arc<dyn ClassLocator> = Arc::new(ModelsLocator);
        config.set_metadata_driver_impl(Arc::new(
            AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
        ));
    });
    orm::create_schema(&entity_manager, &[CATEGORY]).await;

    let schema_validator = SchemaValidator::new(&entity_manager, true);
    schema_validator.validate_mapping().unwrap();

    entity_manager
}

/// `createDummyBlogData()`.
async fn create_dummy_blog_data(
    entity_manager: &EntityManager,
    category_count: usize,
    category_parents_count: usize,
) {
    for h in 0..category_count {
        let mut category_parent: Option<Object> = None;

        for i in 0..category_parents_count {
            let parent = category(&format!("CategoryParent#{i}"), category_parent.as_ref());
            entity_manager.persist(&parent).unwrap();
            category_parent = Some(parent);
        }

        let category = category(&format!("Category#{h}"), category_parent.as_ref());
        entity_manager.persist(&category).unwrap();
    }

    entity_manager.flush().await.unwrap();
    entity_manager.clear().unwrap();
}

#[tokio::test]
async fn testOneToManyWithEagerFetchMode() {
    let entity_manager = create_entity_manager().await;

    create_dummy_blog_data(&entity_manager, 3, 0).await;

    let mut qb = entity_manager.create_query_builder();
    qb.select(vec![s("category")])
        .unwrap()
        .from(CATEGORY, "category", None)
        .unwrap();
    let query = qb.get_query().unwrap();
    query.set_fetch_mode(CATEGORY, "children", FETCH_EAGER);
    let categories = query.get_result(HYDRATE_OBJECT).await.unwrap();

    assert_eq!(list(&categories).len(), 3);
}
