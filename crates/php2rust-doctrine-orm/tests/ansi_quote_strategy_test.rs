//! Port of `AnsiQuoteStrategyTest`, on the test entity manager's platform
//! (`SQLitePlatform`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/AnsiQuoteStrategyTest.php

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
use php2rust_doctrine_orm::mapping::{AnsiQuoteStrategy, QuoteStrategy};
use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;

const CMS_USER: &str = "Doctrine\\Tests\\Models\\CMS\\CmsUser";
const CMS_ADDRESS: &str = "Doctrine\\Tests\\Models\\CMS\\CmsAddress";
const DDC117_ARTICLE: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Article";
const DDC117_ARTICLE_DETAILS: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails";

fn set_up() -> (AnsiQuoteStrategy, SQLitePlatform) {
    models::register();
    (AnsiQuoteStrategy::new(), SQLitePlatform::new())
}

fn create_class_metadata(class_name: &str) -> ClassMetadataInfo {
    let mut class = ClassMetadataInfo::new(class_name, None, None);
    class
        .initialize_reflection(&RuntimeReflectionService::new())
        .unwrap();
    class
}

#[test]
fn testGetColumnName() {
    let (strategy, platform) = set_up();
    let mut class = create_class_metadata(CMS_USER);
    class
        .map_field(php_array! { "fieldName" => "name", "columnName" => "name" })
        .unwrap();
    class
        .map_field(php_array! { "fieldName" => "id", "columnName" => "id", "id" => true })
        .unwrap();

    assert_eq!("id", strategy.get_column_name("id", &class, &platform));
    assert_eq!("name", strategy.get_column_name("name", &class, &platform));
}

#[test]
fn testGetTableName() {
    let (strategy, platform) = set_up();
    let mut class = create_class_metadata(CMS_USER);

    class.set_primary_table(&php_array! { "name" => "cms_user" });
    assert_eq!("cms_user", strategy.get_table_name(&class, &platform));
}

#[test]
fn testJoinTableName() {
    let (strategy, platform) = set_up();
    let mut class = create_class_metadata(CMS_ADDRESS);

    class
        .map_many_to_many(php_array! {
            "fieldName" => "user",
            "targetEntity" => "CmsUser",
            "inversedBy" => "users",
            "joinTable" => php_array! { "name" => "cmsaddress_cmsuser" },
        })
        .unwrap();

    assert_eq!(
        "cmsaddress_cmsuser",
        strategy.get_join_table_name(
            class.association_mappings.get("user").unwrap(),
            &class,
            &platform
        )
    );
}

#[test]
fn testIdentifierColumnNames() {
    let (strategy, platform) = set_up();
    let mut class = create_class_metadata(CMS_ADDRESS);

    class
        .map_field(php_array! { "id" => true, "fieldName" => "id", "columnName" => "id" })
        .unwrap();

    assert_eq!(
        vec!["id"],
        strategy.get_identifier_column_names(&class, &platform)
    );
}

#[test]
fn testColumnAlias() {
    let (strategy, platform) = set_up();
    assert_eq!(
        "columnName_1",
        strategy.get_column_alias("columnName", 1, &platform, None)
    );
}

fn article_details() -> ClassMetadataInfo {
    let mut class = create_class_metadata(DDC117_ARTICLE_DETAILS);
    class
        .map_one_to_one(php_array! {
            "id" => true,
            "fieldName" => "article",
            "targetEntity" => DDC117_ARTICLE,
            "joinColumns" => php_array! [
                php_array! { "name" => "article", "referencedColumnName" => "id" },
            ],
        })
        .unwrap();
    class
}

#[test]
fn testJoinColumnName() {
    let (strategy, platform) = set_up();
    let class = article_details();

    let join_column = &class
        .association_mappings
        .get("article")
        .unwrap()
        .join_columns[0];
    assert_eq!(
        "article",
        strategy.get_join_column_name(join_column, &class, &platform)
    );
}

#[test]
fn testReferencedJoinColumnName() {
    let (strategy, platform) = set_up();
    let cm = article_details();

    let join_column = &cm.association_mappings.get("article").unwrap().join_columns[0];
    assert_eq!(
        "id",
        strategy.get_referenced_join_column_name(join_column, &cm, &platform)
    );
}

#[test]
fn testGetSequenceName() {
    let (strategy, platform) = set_up();
    let mut class = create_class_metadata(CMS_USER);
    let definition = php_array! {
        "sequenceName" => "user_id_seq",
        "allocationSize" => 1,
        "initialValue" => 2,
    };

    class
        .set_sequence_generator_definition(definition.clone())
        .unwrap();

    assert_eq!(
        "user_id_seq",
        strategy.get_sequence_name(&definition, &class, &platform)
    );
}
