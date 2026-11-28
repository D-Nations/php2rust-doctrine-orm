//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5804Test.php
#![allow(non_snake_case)]
#![allow(clippy::unwrap_used, reason = "test helpers")]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{Object, PhpArray, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::platforms::AbstractPlatform;
use php2rust_doctrine_dbal::types::{ConversionError, Type};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use std::sync::Arc;

#[derive(Debug)]
struct GH5804Type;
impl Type for GH5804Type {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH5804Type"
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
        Ok(if value.is_truthy() {
            Value::string("testGh5804DbValue")
        } else {
            Value::Null
        })
    }
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "custom generator callback signature"
)]
fn generate_id(_em: &EntityManager, _entity: Option<&Object>) -> Result<Value, Thrown> {
    Ok(Value::string("test5804"))
}

#[tokio::test]
async fn testTextColumnSaveAndRetrieve2() {
    ticket::add_type("GH5804Type", Arc::new(GH5804Type));
    php2rust_doctrine_orm::id::register_custom_generator(
        &ticket::class("GH5804Generator"),
        generate_id,
    );
    let em = ticket::set_up(&["GH5804Article"]).await;
    let article = ticket::new("GH5804Article");
    orm::set(&article, "text", orm::s("Max"));
    em.persist(&article).unwrap();
    assert_eq!(orm::get(&article, "id"), orm::s("test5804"));
    em.flush().await.unwrap();
    assert_eq!(orm::get(&article, "version"), Value::Int(1));
    orm::set(&article, "text", orm::s("Moritz"));
    em.persist(&article).unwrap();
    em.flush().await.unwrap();
    assert_eq!(orm::get(&article, "version"), Value::Int(2));
}
