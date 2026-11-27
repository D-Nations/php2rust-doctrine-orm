//! Ticket/DDC5684Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC5684ObjectIdType` and `DDC5684ObjectId` are Rust types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC5684Test.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, PhpObject, PhpString, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};

use orm::*;
use ticket::*;

const DDC5684_OBJECT_ID_TYPE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC5684ObjectIdType";
const DDC5684_OBJECT_ID: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC5684ObjectId";

/// `DDC5684ObjectId implements Stringable`.
#[derive(Debug)]
struct DDC5684ObjectId {
    value: Value,
}

impl PhpObject for DDC5684ObjectId {
    fn class(&self) -> &str {
        DDC5684_OBJECT_ID
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("value", self.value.clone());
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        self.value.to_php_string()
    }
}

/// `new DDC5684ObjectId($value)`.
fn object_id(value: Value) -> Value {
    let id: Object = Arc::new(DDC5684ObjectId { value });
    Value::Object(id)
}

/// `$id->value`.
fn value_of(id: &Value) -> Value {
    let Value::Object(object) = id else {
        panic!("a DDC5684ObjectId")
    };
    object
        .properties()
        .get(&"value".into())
        .cloned()
        .unwrap_or(Value::Null)
}

/// `DDC5684ObjectIdType extends DBALTypes\Type`.
#[derive(Debug)]
struct DDC5684ObjectIdType;

impl Type for DDC5684ObjectIdType {
    fn class(&self) -> &'static str {
        DDC5684_OBJECT_ID_TYPE
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(object_id(value.clone()))
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(value_of(value))
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_integer_type_declaration_sql(column)
    }
}

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    ticket::add_type(DDC5684_OBJECT_ID_TYPE, Arc::new(DDC5684ObjectIdType));
    ticket::set_up(&["DDC5684Object"]).await
}

fn assert_object_id(value: &Value) {
    assert!(
        matches!(value, Value::Object(object) if object.class() == DDC5684_OBJECT_ID),
        "{value:?} is not a {DDC5684_OBJECT_ID}"
    );
}

#[tokio::test]
async fn testAutoIncrementIdWithCustomType() {
    let em = set_up().await;
    let object = new("DDC5684Object");
    em.persist(&object).unwrap();
    em.flush().await.unwrap();

    assert_object_id(&get(&object, "id"));
}

#[tokio::test]
async fn testFetchObjectWithAutoIncrementedCustomType() {
    let em = set_up().await;
    let object = new("DDC5684Object");
    em.persist(&object).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let raw_id = value_of(&get(&object, "id"));
    let object = find(&em, "DDC5684Object", object_id(raw_id.clone()))
        .await
        .expect("the object");

    assert_object_id(&get(&object, "id"));
    assert_loose_eq(&raw_id, &value_of(&get(&object, "id")));
}
