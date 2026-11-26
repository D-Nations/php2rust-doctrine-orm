//! Ticket/GH10747Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH10747CustomIdObjectHashType` and `CustomIdObject` are Rust
//! types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10747Test.php

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
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::*;
use ticket::*;

const GH10747_CUSTOM_ID_OBJECT_HASH_TYPE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH10747CustomIdObjectHashType";

/// `Doctrine\Tests\DbalTypes\CustomIdObject implements Stringable`.
#[derive(Debug)]
struct CustomIdObject {
    id: PhpString,
}

impl PhpObject for CustomIdObject {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\DbalTypes\\CustomIdObject"
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("id", Value::String(self.id.clone()));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(self.id.clone())
    }
}

/// `new CustomIdObject($id)`.
fn custom_id_object(id: impl Into<PhpString>) -> Value {
    let id: Object = Arc::new(CustomIdObject { id: id.into() });
    Value::Object(id)
}

/// `GH10747CustomIdObjectHashType extends DBALType`.
#[derive(Debug)]
struct GH10747CustomIdObjectHashType;

impl Type for GH10747CustomIdObjectHashType {
    fn class(&self) -> &'static str {
        GH10747_CUSTOM_ID_OBJECT_HASH_TYPE
    }

    fn convert_to_database_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let Value::Object(object) = value else {
            panic!("a CustomIdObject")
        };
        let id = object
            .properties()
            .get(&"id".into())
            .cloned()
            .unwrap_or(Value::Null);
        let id = id
            .to_php_string()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Ok(Value::from(format!("{id}_test").as_str()))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        let text = value
            .to_php_string()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Ok(custom_id_object(text.replace("_test", "")))
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_string_type_declaration_sql(column)
    }
}

/// `new GH10747Credit($article, $name)`.
fn credit(article: &Object, name: &str) -> Object {
    let credit = new("GH10747Credit");
    set_obj(&credit, "article", article);
    set(&credit, "name", s(name));
    credit
}

#[tokio::test]
async fn testOrphanedOneToManyDeletesCollection() {
    if !php2rust_doctrine_dbal::types::has_type(GH10747_CUSTOM_ID_OBJECT_HASH_TYPE) {
        ticket::add_type(
            GH10747_CUSTOM_ID_OBJECT_HASH_TYPE,
            Arc::new(GH10747CustomIdObjectHashType),
        );
    }
    let em = ticket::set_up(&["GH10747Article", "GH10747Credit"]).await;

    let object = new("GH10747Article");
    set(&object, "id", custom_id_object("article"));
    set(
        &object,
        "credits",
        ArrayCollectionObject::value(PhpArray::new()),
    );

    let credit_one = credit(&object, "credit1");
    let credit_two = credit(&object, "credit2");

    set(
        &object,
        "credits",
        ArrayCollectionObject::value(PhpArray::list(vec![
            Value::Object(credit_one.clone()),
            Value::Object(credit_two.clone()),
        ])),
    );

    em.persist(&object).unwrap();
    em.persist(&credit_one).unwrap();
    em.persist(&credit_two).unwrap();
    em.flush().await.unwrap();

    let id = get(&object, "id");

    let object2 = find(&em, "GH10747Article", id).await.expect("the article");

    let credit_three = credit(&object2, "credit3");

    set(
        &object2,
        "credits",
        ArrayCollectionObject::value(PhpArray::list(vec![Value::Object(credit_three.clone())])),
    );

    em.persist(&object2).unwrap();
    em.persist(&credit_three).unwrap();
    em.flush().await.unwrap();

    let mut qb = em.create_query_builder();
    qb.select(vec![s("c.id")])
        .unwrap()
        .from(&class("GH10747Credit"), "c", None)
        .unwrap();
    let current_database_credits = qb.get_query().unwrap().execute(None, None).await.unwrap();

    assert_eq!(1, list(&current_database_credits).len());
}
