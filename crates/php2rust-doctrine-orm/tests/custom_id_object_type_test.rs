//! Port of tests/Tests/ORM/Functional/CustomIdObjectTypeTest.php, on
//! `SQLite` with the recorded models of the `custom_id_object_type` model
//! set. `Doctrine\Tests\DbalTypes\CustomIdObject` (a `Stringable` with a
//! public `$id`) and `CustomIdObjectType` are Rust types of this file.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CustomIdObjectTypeTest.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpObject, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{
    ConversionError, Type, TypeArgument, add_type, has_type, override_type,
};
use php2rust_doctrine_orm::collection_object::{self, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, list, new_object, same_value, set, set_obj};

const PARENT: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeParent";
const CHILD: &str = "Doctrine\\Tests\\Models\\CustomType\\CustomIdObjectTypeChild";

/// `Doctrine\Tests\DbalTypes\CustomIdObject`.
#[derive(Debug)]
struct CustomIdObject {
    id: String,
}

impl CustomIdObject {
    /// `new CustomIdObject($id)`.
    fn value(id: &str) -> Value {
        Value::Object(Arc::new(Self { id: id.to_owned() }))
    }
}

impl PhpObject for CustomIdObject {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\CustomIdObject"
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("id", Value::string(self.id.as_str()));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(PhpString::from(self.id.as_str()))
    }

    fn is_stringable(&self) -> bool {
        true
    }

    fn instance_of(&self, class: &str) -> bool {
        class.eq_ignore_ascii_case(self.class()) || class.eq_ignore_ascii_case("Stringable")
    }
}

/// `Doctrine\Tests\DbalTypes\CustomIdObjectType`.
#[derive(Debug)]
struct CustomIdObjectType;

impl Type for CustomIdObjectType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\CustomIdObjectType"
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let Value::Object(object) = value else {
            return Ok(value.clone());
        };
        Ok(object.property(b"id").unwrap_or(Value::Null))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let id = value.to_php_string().unwrap_or_default();
        Ok(CustomIdObject::value(&id.to_string_lossy()))
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }
}

async fn set_up() -> Arc<EntityManager> {
    let type_ = || TypeArgument::Instance(Arc::new(CustomIdObjectType));
    if has_type("CustomIdObject") {
        override_type("CustomIdObject", type_()).unwrap();
    } else if add_type("CustomIdObject", type_()).is_err() {
        // Another test thread may have added it meanwhile.
        override_type("CustomIdObject", type_()).unwrap();
    }

    orm::set_up(&[PARENT, CHILD]).await
}

/// `new CustomIdObjectTypeParent($id)`.
fn parent(id: &str) -> Object {
    let parent = new_object(PARENT);
    set(&parent, "id", CustomIdObject::value(id));
    set(
        &parent,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    parent
}

/// `new CustomIdObjectTypeChild($id, $parent)`.
fn child(id: &str, parent: &Object) -> Object {
    let child = new_object(CHILD);
    set(&child, "id", CustomIdObject::value(id));
    set_obj(&child, "parent", parent);
    child
}

#[tokio::test]
async fn testFindByCustomIdObject() {
    let em = set_up().await;
    let parent = parent("foo");

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();

    let result = em.find(PARENT, get(&parent, "id")).await.unwrap();

    assert!(result.is_some_and(|result| Arc::ptr_eq(&result, &parent)));
}

/// DDC-3622, 1336
#[tokio::test]
async fn testFetchJoinCustomIdObject() {
    let em = set_up().await;
    let parent = parent("foo");

    collection_object::add(
        &get(&parent, "children"),
        Value::Object(child("bar", &parent)),
    )
    .unwrap();

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();

    let result = em
        .create_query(&format!(
            "SELECT parent, children FROM {PARENT} parent LEFT JOIN parent.children children"
        ))
        .get_object_result()
        .await
        .unwrap();

    let result = list(&result);
    assert_eq!(result.len(), 1);
    assert!(same_value(&result[0], &parent));
}

/// DDC-3622, 1336
#[tokio::test]
async fn testFetchJoinWhereCustomIdObject() {
    let em = set_up().await;
    let parent = parent("foo");

    let child = child("bar", &parent);
    collection_object::add(&get(&parent, "children"), Value::Object(child.clone())).unwrap();

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();

    // note: hydration is willingly broken in this example:
    let query = em.create_query(&format!(
        "SELECT parent, children FROM {PARENT} parent LEFT JOIN parent.children children WHERE children.id = ?1"
    ));
    // `$parent->children->first()->id`.
    query.set_parameter(1, get(&child, "id"), None);
    let result = query.get_object_result().await.unwrap();

    let result = list(&result);
    assert_eq!(result.len(), 1);
    assert!(same_value(&result[0], &parent));
}
