//! Ticket/DDC2579Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC2579Type` and `DDC2579Id` are Rust types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2579Test.php

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

/// `DDC2579Type extends StringType`.
#[derive(Debug)]
struct DDC2579Type;

impl DDC2579Type {
    const NAME: &str = "ddc2579";
}

impl Type for DDC2579Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2579Type"
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
        Ok(ddc2579_id(value.to_php_string().unwrap_or_default()))
    }
}

/// `DDC2579Id implements Stringable`.
#[derive(Debug)]
struct DDC2579Id {
    val: PhpString,
}

impl PhpObject for DDC2579Id {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC2579Id"
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(self.val.clone())
    }
}

/// `new DDC2579Id($val)`.
fn ddc2579_id(val: impl Into<PhpString>) -> Value {
    let id: Object = Arc::new(DDC2579Id { val: val.into() });
    Value::Object(id)
}

#[tokio::test]
async fn testIssue() {
    ticket::add_type(DDC2579Type::NAME, Arc::new(DDC2579Type));
    let em = ticket::set_up(&["DDC2579Entity", "DDC2579EntityAssoc", "DDC2579AssocAssoc"]).await;

    let id = ddc2579_id("foo");
    let assoc = new("DDC2579AssocAssoc");
    set(&assoc, "associationId", id);
    let assoc_assoc = new("DDC2579EntityAssoc");
    set_obj(&assoc_assoc, "assocAssoc", &assoc);
    let entity = new("DDC2579Entity");
    set(&entity, "value", Value::Int(0));
    set(&entity, "id", get(&assoc, "associationId"));
    set_obj(&entity, "assoc", &assoc_assoc);
    let repository = em.get_repository(&class("DDC2579Entity")).unwrap();

    em.persist(&assoc).unwrap();
    em.persist(&assoc_assoc).unwrap();
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    // $entity->value++
    let Value::Int(value) = get(&entity, "value") else {
        panic!("value is an int")
    };
    set(&entity, "value", Value::Int(value + 1));

    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let id = get(&entity, "id");
    let value = get(&entity, "value");
    let mut criteria = PhpArray::new();
    criteria.insert("assoc", Value::Object(assoc.clone()));
    criteria.insert("id", id);
    let entity = repository
        .find_one_by(&criteria, None)
        .await
        .unwrap()
        .expect("the entity");

    assert_instance_of("DDC2579Entity", &Value::Object(entity.clone()));
    assert_loose_eq(&value, &get(&entity, "value"));

    em.remove(&entity).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert!(
        repository
            .find_one_by(&criteria, None)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(0, repository.find_all().await.unwrap().len());
}
