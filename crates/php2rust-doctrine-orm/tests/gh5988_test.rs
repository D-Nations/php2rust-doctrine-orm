//! Ticket/GH5988Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `GH5988CustomIdObjectHashType` and `CustomIdObject` are Rust
//! types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5988Test.php

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

const GH5988_CUSTOM_ID_OBJECT_HASH_TYPE: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH5988CustomIdObjectHashType";

/// `Doctrine\Tests\DbalTypes\CustomIdObject implements Stringable`.
#[derive(Debug)]
struct CustomIdObject {
    id: PhpString,
}

impl PhpObject for CustomIdObject {
    fn class(&self) -> &str {
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

/// `GH5988CustomIdObjectHashType extends DBALType`.
#[derive(Debug)]
struct GH5988CustomIdObjectHashType;

impl Type for GH5988CustomIdObjectHashType {
    fn class(&self) -> &'static str {
        GH5988_CUSTOM_ID_OBJECT_HASH_TYPE
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

#[tokio::test]
async fn testDelete() {
    if !php2rust_doctrine_dbal::types::has_type(GH5988_CUSTOM_ID_OBJECT_HASH_TYPE) {
        ticket::add_type(
            GH5988_CUSTOM_ID_OBJECT_HASH_TYPE,
            Arc::new(GH5988CustomIdObjectHashType),
        );
    }
    let em = ticket::set_up(&[
        "GH5988CustomIdObjectTypeParent",
        "GH5988CustomIdObjectTypeChild",
    ])
    .await;

    let object = new("GH5988CustomIdObjectTypeChild");
    set(&object, "id", custom_id_object("foo"));
    set(&object, "name", s("Test"));
    em.persist(&object).unwrap();
    em.flush().await.unwrap();

    let id = get(&object, "id");
    let object2 = find(&em, "GH5988CustomIdObjectTypeChild", id.clone())
        .await
        .expect("the child");

    em.remove(&object2).await.unwrap();
    em.flush().await.unwrap();

    assert!(
        find(&em, "GH5988CustomIdObjectTypeChild", id)
            .await
            .is_none()
    );
}
