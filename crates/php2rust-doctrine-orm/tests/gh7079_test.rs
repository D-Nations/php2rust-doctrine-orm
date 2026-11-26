//! Ticket/GH7079Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; the platform is DBAL's `SQLitePlatform`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7079Test.php

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

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_dbal::platforms::SQLitePlatform;
use php2rust_doctrine_orm::mapping::class_metadata::ClassMetadataInfo;
use php2rust_doctrine_orm::mapping::{DefaultQuoteStrategy, QuoteStrategy};
use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;

use ticket::*;

fn set_up() -> (DefaultQuoteStrategy, SQLitePlatform) {
    models::register();
    (DefaultQuoteStrategy::new(), SQLitePlatform::new())
}

/// `getTableFullName($table)`.
fn get_table_full_name(table: &PhpArray) -> String {
    let text = |key: &str| match table.get(&key.into()) {
        Some(Value::String(s)) => s.to_string(),
        other => panic!("{key}: {other:?}"),
    };
    format!("{}.{}", text("schema"), text("name"))
}

/// `createClassMetadata($className)`.
fn create_class_metadata(class_name: &str) -> ClassMetadataInfo {
    let mut cm = ClassMetadataInfo::new(class_name, None, None);
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    cm
}

#[test]
fn testGetTableName() {
    let (strategy, platform) = set_up();
    let table = php_array! {
        "name" => "cms_user",
        "schema" => "cms",
    };

    let mut cm = create_class_metadata(&class("GH7079CmsUser"));
    cm.set_primary_table(&table);

    assert_eq!(
        get_table_full_name(&table),
        strategy.get_table_name(&cm, &platform)
    );
}

#[test]
fn testJoinTableName() {
    let (strategy, platform) = set_up();
    let table = php_array! {
        "name" => "cmsaddress_cmsuser",
        "schema" => "cms",
    };

    let mut cm = create_class_metadata(&class("GH7079CmsAddress"));
    cm.map_many_to_many(php_array! {
        "fieldName" => "user",
        "targetEntity" => "DDC7079CmsUser",
        "inversedBy" => "users",
        "joinTable" => Value::Array(table.clone()),
    })
    .unwrap();

    assert_eq!(
        get_table_full_name(&table),
        strategy.get_join_table_name(
            cm.association_mappings.get("user").unwrap(),
            &cm,
            &platform
        )
    );
}
