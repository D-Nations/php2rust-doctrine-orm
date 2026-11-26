//! Ticket/DDC3785Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; `DDC3785AssetIdType` and the `DDC3785AssetId` value are Rust
//! types).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3785Test.php

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
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `DDC3785AssetId implements Stringable`.
#[derive(Debug)]
struct DDC3785AssetId {
    id: PhpString,
}

impl PhpObject for DDC3785AssetId {
    fn class(&self) -> &str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC3785AssetId"
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn to_php_string(&self) -> Option<PhpString> {
        Some(self.id.clone())
    }
}

/// `new DDC3785AssetId($id)`.
fn asset_id(id: impl Into<PhpString>) -> Value {
    let id: Object = Arc::new(DDC3785AssetId { id: id.into() });
    Value::Object(id)
}

/// `DDC3785AssetIdType extends Type`.
#[derive(Debug)]
struct DDC3785AssetIdType;

impl Type for DDC3785AssetIdType {
    fn class(&self) -> &'static str {
        "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC3785AssetIdType"
    }

    fn get_sql_declaration(
        &self,
        column: &PhpArray,
        platform: &dyn AbstractPlatform,
    ) -> Result<PhpString, php2rust_doctrine_dbal::Exception> {
        platform.get_guid_type_declaration_sql(column)
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
        Ok(asset_id(value.to_php_string().unwrap_or_default()))
    }
}

/// `new DDC3785Attribute($name, $value)`.
fn attribute(name: &str, value: &str) -> Object {
    let attribute = new("DDC3785Attribute");
    set(&attribute, "name", s(name));
    set(&attribute, "value", s(value));
    attribute
}

#[tokio::test]
async fn testOwningValueObjectIdIsCorrectlyTransformedWhenRemovingOrphanedChildEntities() {
    ticket::add_type("ddc3785_asset_id", Arc::new(DDC3785AssetIdType));
    let em = ticket::set_up(&["DDC3785Asset", "DDC3785AssetId", "DDC3785Attribute"]).await;

    let id = asset_id("919609ba-57d9-4a13-be1d-d202521e858a");

    let attribute1 = attribute("foo1", "bar1");
    let attribute2 = attribute("foo2", "bar2");

    let asset = new("DDC3785Asset");
    set(&asset, "id", id);
    set(
        &asset,
        "attributes",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    for attribute in [&attribute1, &attribute2] {
        coll::add(&get(&asset, "attributes"), Value::Object(attribute.clone())).unwrap();
    }
    em.persist(&asset).unwrap();
    em.flush().await.unwrap();

    coll::remove_element(
        &get(&asset, "attributes"),
        &Value::Object(attribute1.clone()),
    )
    .await
    .unwrap();

    let id_to_be_removed = get(&attribute1, "id");

    em.persist(&asset).unwrap();
    em.flush().await.unwrap();

    assert!(
        find(&em, "DDC3785Attribute", id_to_be_removed)
            .await
            .is_none()
    );
    assert!(
        find(&em, "DDC3785Attribute", get(&attribute2, "id"))
            .await
            .is_some()
    );
}
