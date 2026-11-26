//! Ticket/GH5887Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH5887CustomIdObjectType` and `GH5887CustomIdObject` are Rust
//! types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5887Test.php

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

/// `GH5887CustomIdObject implements Stringable`.
#[derive(Debug)]
struct GH5887CustomIdObject {
    id: i64,
}

impl PhpObject for GH5887CustomIdObject {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH5887CustomIdObject"
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("id", Value::Int(self.id));
        properties
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(PhpString::from("non existing id"))
    }
}

/// `new GH5887CustomIdObject($id)`.
fn custom_id_object(id: i64) -> Value {
    let id: Object = Arc::new(GH5887CustomIdObject { id });
    Value::Object(id)
}

/// `$id->getId()`.
fn get_id(id: &Value) -> Value {
    let Value::Object(object) = id else {
        panic!("a GH5887CustomIdObject")
    };
    object
        .properties()
        .get(&"id".into())
        .cloned()
        .unwrap_or(Value::Null)
}

/// `GH5887CustomIdObjectType extends StringType`.
#[derive(Debug)]
struct GH5887CustomIdObjectType;

impl GH5887CustomIdObjectType {
    const NAME: &str = "GH5887CustomIdObject";
}

impl Type for GH5887CustomIdObjectType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH5887CustomIdObjectType"
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
        Ok(get_id(value))
    }

    fn convert_to_php_value(
        &self,
        value: &Value,
        _platform: &dyn AbstractPlatform,
    ) -> Result<Value, ConversionError> {
        Ok(custom_id_object(php_value::ops::to_int(value)))
    }
}

#[tokio::test]
async fn testLazyLoadsForeignEntitiesInOneToOneRelationWhileHavingCustomIdObject() {
    ticket::add_type(
        GH5887CustomIdObjectType::NAME,
        Arc::new(GH5887CustomIdObjectType),
    );
    let em = ticket::set_up(&["GH5887Cart", "GH5887Customer"]).await;

    let customer_id = custom_id_object(1);
    let customer = new("GH5887Customer");
    set(&customer, "id", customer_id.clone());

    let cart_id = 2;
    let cart = new("GH5887Cart");
    set(&cart, "id", Value::Int(cart_id));
    // $cart->setCustomer($customer)
    set_obj(&cart, "customer", &customer);
    set_obj(&customer, "cart", &cart);

    em.persist(&customer).unwrap();
    em.persist(&cart).unwrap();
    em.flush().await.unwrap();

    // Clearing cached entities
    em.clear().unwrap();

    let customer_repository = em.get_repository(&class("GH5887Customer")).unwrap();
    let mut qb = customer_repository.create_query_builder("c", None).unwrap();
    qb.r#where(vec![s("c.id = :id")]).unwrap();
    qb.set_parameter("id", get_id(&customer_id), None);
    let customer = qb
        .get_query()
        .unwrap()
        .get_one_or_null_result(None)
        .await
        .unwrap();

    assert_instance_of("GH5887Customer", &customer);
    assert_instance_of("GH5887Cart", &get(&obj(&customer), "cart"));
}
