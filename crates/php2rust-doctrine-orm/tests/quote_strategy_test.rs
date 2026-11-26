//! Port of `QuoteStrategyTest`, on the test entity manager's platform
//! (`SQLitePlatform`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/QuoteStrategyTest.php

#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;

use php_value::php_array;
use php2rust_doctrine_dbal::platforms::SQLitePlatform;
use php2rust_doctrine_orm::mapping::class_metadata::ClassMetadataInfo;
use php2rust_doctrine_orm::mapping::{DefaultQuoteStrategy, QuoteStrategy};
use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;

const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
const CMS_ADDRESS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddress";
const DDC117_ARTICLE: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Article";
const DDC117_ARTICLE_DETAILS: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails";

fn set_up() -> (DefaultQuoteStrategy, SQLitePlatform) {
    models::register();
    (DefaultQuoteStrategy::new(), SQLitePlatform::new())
}

fn create_class_metadata(class_name: &str) -> ClassMetadataInfo {
    let mut cm = ClassMetadataInfo::new(class_name, None, None);
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    cm
}

#[test]
fn testGetColumnName() {
    let (strategy, platform) = set_up();
    let mut cm = create_class_metadata(CMS_USER);
    cm.map_field(php_array! { "fieldName" => "name", "columnName" => "`name`" })
        .unwrap();
    cm.map_field(php_array! { "fieldName" => "id", "columnName" => "id" })
        .unwrap();

    assert_eq!("id", strategy.get_column_name("id", &cm, &platform));
    assert_eq!("\"name\"", strategy.get_column_name("name", &cm, &platform));
}

#[test]
fn testGetTableName() {
    let (strategy, platform) = set_up();
    let mut cm = create_class_metadata(CMS_USER);
    cm.set_primary_table(&php_array! { "name" => "`cms_user`" });
    assert_eq!("\"cms_user\"", strategy.get_table_name(&cm, &platform));

    let mut cm = ClassMetadataInfo::new(CMS_USER, None, None);
    cm.initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    cm.set_primary_table(&php_array! { "name" => "cms_user" });
    assert_eq!("cms_user", strategy.get_table_name(&cm, &platform));
}

#[test]
fn testJoinTableName() {
    let (strategy, platform) = set_up();
    let mut cm1 = create_class_metadata(CMS_ADDRESS);
    let mut cm2 = create_class_metadata(CMS_ADDRESS);

    cm1.map_many_to_many(php_array! {
        "fieldName" => "user",
        "targetEntity" => "CmsUser",
        "inversedBy" => "users",
        "joinTable" => php_array! { "name" => "`cmsaddress_cmsuser`" },
    })
    .unwrap();

    cm2.map_many_to_many(php_array! {
        "fieldName" => "user",
        "targetEntity" => "CmsUser",
        "inversedBy" => "users",
        "joinTable" => php_array! { "name" => "cmsaddress_cmsuser" },
    })
    .unwrap();

    assert_eq!(
        "\"cmsaddress_cmsuser\"",
        strategy.get_join_table_name(
            cm1.association_mappings.get("user").unwrap(),
            &cm1,
            &platform
        )
    );
    assert_eq!(
        "cmsaddress_cmsuser",
        strategy.get_join_table_name(
            cm2.association_mappings.get("user").unwrap(),
            &cm2,
            &platform
        )
    );
}

#[test]
fn testIdentifierColumnNames() {
    let (strategy, platform) = set_up();
    let mut cm1 = create_class_metadata(CMS_ADDRESS);
    let mut cm2 = create_class_metadata(CMS_ADDRESS);

    cm1.map_field(php_array! { "id" => true, "fieldName" => "id", "columnName" => "`id`" })
        .unwrap();

    cm2.map_field(php_array! { "id" => true, "fieldName" => "id", "columnName" => "id" })
        .unwrap();

    assert_eq!(
        vec!["\"id\""],
        strategy.get_identifier_column_names(&cm1, &platform)
    );
    assert_eq!(
        vec!["id"],
        strategy.get_identifier_column_names(&cm2, &platform)
    );
}

#[test]
fn testColumnAlias() {
    let (strategy, platform) = set_up();
    assert_eq!(
        "columnName_0",
        strategy.get_column_alias("columnName", 0, &platform, None)
    );
    assert_eq!(
        "column_name_1",
        strategy.get_column_alias("column_name", 1, &platform, None)
    );
    assert_eq!(
        "COLUMN_NAME_2",
        strategy.get_column_alias("COLUMN_NAME", 2, &platform, None)
    );
    assert_eq!(
        "COLUMNNAME_3",
        strategy.get_column_alias("COLUMN-NAME-", 3, &platform, None)
    );
}

fn article_details(name: &str, referenced_column_name: &str) -> ClassMetadataInfo {
    let mut cm = create_class_metadata(DDC117_ARTICLE_DETAILS);
    cm.map_one_to_one(php_array! {
        "id" => true,
        "fieldName" => "article",
        "targetEntity" => DDC117_ARTICLE,
        "joinColumns" => php_array! [
            php_array! { "name" => name, "referencedColumnName" => referenced_column_name },
        ],
    })
    .unwrap();
    cm
}

#[test]
fn testQuoteIdentifierJoinColumns() {
    let (strategy, platform) = set_up();
    let cm = article_details("`article`", "article");

    assert_eq!(
        vec!["\"article\""],
        strategy.get_identifier_column_names(&cm, &platform)
    );
}

#[test]
fn testJoinColumnName() {
    let (strategy, platform) = set_up();
    let cm = article_details("`article`", "article");

    let join_column = &cm.association_mappings.get("article").unwrap().join_columns[0];
    assert_eq!(
        "\"article\"",
        strategy.get_join_column_name(join_column, &cm, &platform)
    );
}

#[test]
fn testReferencedJoinColumnName() {
    let (strategy, platform) = set_up();
    let cm = article_details("`article`", "id");

    let join_column = &cm.association_mappings.get("article").unwrap().join_columns[0];
    assert_eq!(
        "\"id\"",
        strategy.get_referenced_join_column_name(join_column, &cm, &platform)
    );
}
