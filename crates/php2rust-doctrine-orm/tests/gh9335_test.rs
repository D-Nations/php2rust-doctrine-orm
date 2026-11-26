//! Ticket/GH9335Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH9335IntObjectType` and `GH9335IntObject` are Rust types.
//! `requiresSQLCommentHint()` has no counterpart in DBAL 4).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9335Test.php

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
use php2rust_doctrine_dbal::ParameterType;
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};

use orm::*;
use ticket::*;

const GH9335_INT_OBJECT_TYPE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH9335IntObjectType";
const GH9335_INT_OBJECT: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH9335IntObject";

/// `GH9335IntObject`, with `__toString()`.
#[derive(Debug)]
struct GH9335IntObject {
    wrapped_int: i64,
}

impl PhpObject for GH9335IntObject {
    fn class(&self) -> &str {
        GH9335_INT_OBJECT
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("wrappedInt", Value::Int(self.wrapped_int));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(PhpString::from(self.wrapped_int.to_string()))
    }
}

/// `new GH9335IntObject($wrappedInt)`.
fn int_object(wrapped_int: i64) -> Value {
    let object: Object = Arc::new(GH9335IntObject { wrapped_int });
    Value::Object(object)
}

/// `GH9335IntObjectType extends Type`.
#[derive(Debug)]
struct GH9335IntObjectType;

impl Type for GH9335IntObjectType {
    fn class(&self) -> &'static str {
        GH9335_INT_OBJECT_TYPE
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_integer_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let Value::Object(object) = value else {
            panic!("a GH9335IntObject")
        };
        Ok(object
            .properties()
            .get(&"wrappedInt".into())
            .cloned()
            .unwrap_or(Value::Null))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(int_object(php_value::ops::to_int(value)))
    }

    fn get_binding_type(&self) -> ParameterType {
        ParameterType::Integer
    }
}

#[tokio::test]
async fn testFlattenIdentifierWithObjectId() {
    if !php2rust_doctrine_dbal::types::has_type(GH9335_INT_OBJECT_TYPE) {
        ticket::add_type(GH9335_INT_OBJECT_TYPE, Arc::new(GH9335IntObjectType));
    }
    let em = ticket::set_up(&["GH9335Book", "GH9335Author"]).await;

    let author = new("GH9335Author");
    set(&author, "name", s("Douglas Adams"));
    let book = new("GH9335Book");
    set(&book, "id", int_object(42));
    set(&book, "title", s("The Hitchhiker's Guide to the Galaxy"));
    // $book->setAuthor($author)
    set_obj(&book, "author", &author);
    set_obj(&author, "book", &book);

    em.persist(&author).unwrap();
    em.persist(&book).unwrap();
    em.flush().await.unwrap();

    em.refresh(&book).await.unwrap();

    assert!(
        matches!(get(&book, "id"), Value::Object(id) if id.class() == GH9335_INT_OBJECT),
        "the id is a {GH9335_INT_OBJECT}"
    );
}
