//! Ticket/DDC1998Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC1998Type` and `DDC1998Id` are Rust types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1998Test.php

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

/// `DDC1998Type extends StringType`.
#[derive(Debug)]
struct DDC1998Type;

impl DDC1998Type {
    const NAME: &str = "ddc1998";
}

impl Type for DDC1998Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1998Type"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(Value::String(
            value.to_php_string_or_stringable().unwrap_or_default(),
        ))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(ddc1998_id(value.to_php_string().unwrap_or_default()))
    }
}

/// `DDC1998Id implements Stringable`.
#[derive(Debug)]
struct DDC1998Id {
    val: PhpString,
}

impl PhpObject for DDC1998Id {
    fn class(&self) -> &str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1998Id"
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(self.val.clone())
    }
}

/// `new DDC1998Id($val)`.
fn ddc1998_id(val: impl Into<PhpString>) -> Value {
    let id: Object = Arc::new(DDC1998Id { val: val.into() });
    Value::Object(id)
}

#[tokio::test]
async fn testSqlConversionAsIdentifier() {
    ticket::add_type(DDC1998Type::NAME, Arc::new(DDC1998Type));
    let em = ticket::set_up(&["DDC1998Entity"]).await;

    let entity = new("DDC1998Entity");
    set(&entity, "id", ddc1998_id("foo"));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    // $entity->num++
    let Value::Int(num) = get(&entity, "num") else {
        panic!("num is an int")
    };
    set(&entity, "num", Value::Int(num + 1));
    em.flush().await.unwrap();

    em.remove(&entity).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = find(&em, "DDC1998Entity", get(&entity, "id")).await;
    assert!(found.is_none());

    let found = find(&em, "DDC1998Entity", s("foo")).await;
    assert!(found.is_none());

    assert_eq!(0, find_all(&em, "DDC1998Entity").await.len());
}
