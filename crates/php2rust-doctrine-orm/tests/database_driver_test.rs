//! Port of `DatabaseDriverTest`, on `SQLite`, over the cms model set.
//! The tables are built with DBAL's `TableEditor`, as upstream does when
//! it exists. `SQLite` supports unsigned integers, so the unsigned
//! assertion runs.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/DatabaseDriverTest.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/database_driver_test_case.rs"]
mod database_driver_test_case;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::{Key, PhpArray, Value, php_array};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use database_driver_test_case::{
    column, convert_to_class_metadata, drop_and_create_table, extract_class_metadata, foreign_key,
    metadata, table,
};

/// `setUp()`: `useModelSet('cms')`.
async fn set_up() -> Arc<EntityManager> {
    models::register();
    orm::set_up(&orm::CMS_MODELS).await
}

fn none() -> PhpArray {
    PhpArray::new()
}

#[tokio::test]
async fn testIssue2059() {
    let em = set_up().await;
    let user = table(
        "ddc2059_user",
        vec![column("id", "integer", &none())],
        &["id"],
        vec![],
    );

    let project = table(
        "ddc2059_project",
        vec![
            column("id", "integer", &none()),
            column("user_id", "integer", &none()),
            column("user", "string", &none()),
        ],
        &["id"],
        vec![foreign_key(&["user_id"], "ddc2059_user", &["id"])],
    );

    let metadatas = convert_to_class_metadata(&em, vec![project, user], vec![]).await;

    let project = metadata(&metadatas, "Ddc2059Project");
    assert!(project.field_mappings.get("user").is_some());
    assert!(project.association_mappings.get("user2").is_some());
}

#[tokio::test]
async fn testLoadMetadataFromDatabase() {
    let em = set_up().await;
    let table = table(
        "dbdriver_foo",
        vec![
            column("id", "integer", &none()),
            column(
                "bar",
                "string",
                &php_array! { "notnull" => false, "length" => 200_i64 },
            ),
        ],
        &["id"],
        vec![],
    );

    drop_and_create_table(&em, &table).await;

    let metadatas = extract_class_metadata(&em, &["DbdriverFoo"]).await;

    let metadata = metadata(&metadatas, "DbdriverFoo");

    let id = metadata.field_mappings.get("id").unwrap();
    assert_eq!("id", id.field_name);
    assert_eq!("id", id.column_name.to_lowercase());
    assert_eq!("integer", id.r#type);

    let bar = metadata.field_mappings.get("bar").unwrap();
    assert_eq!("bar", bar.field_name);
    assert_eq!("bar", bar.column_name.to_lowercase());
    assert_eq!("string", bar.r#type);
    assert_eq!(Some(200), bar.length);
    assert_eq!(Some(true), bar.nullable);
}

#[tokio::test]
async fn testLoadMetadataWithForeignKeyFromDatabase() {
    let em = set_up().await;
    let table_b = table(
        "dbdriver_bar",
        vec![column("id", "integer", &none())],
        &["id"],
        vec![],
    );

    let table_a = table(
        "dbdriver_baz",
        vec![
            column("id", "integer", &none()),
            column("bar_id", "integer", &none()),
        ],
        &["id"],
        vec![foreign_key(&["bar_id"], "dbdriver_bar", &["id"])],
    );

    drop_and_create_table(&em, &table_b).await;
    drop_and_create_table(&em, &table_a).await;

    let metadatas = extract_class_metadata(&em, &["DbdriverBar", "DbdriverBaz"]).await;

    let baz_metadata = metadata(&metadatas, "DbdriverBaz");

    assert!(
        baz_metadata.field_mappings.get("barId").is_none(),
        "The foreign Key field should not be inflected as 'barId' field, its an association."
    );
    assert!(baz_metadata.field_mappings.get("id").is_some());

    let bar = baz_metadata
        .association_mappings
        .iter()
        .find(|(name, _)| name.to_lowercase() == "bar")
        .map(|(_, mapping)| mapping)
        .unwrap();
    assert!(bar.is_many_to_one());
}

#[tokio::test]
async fn testDetectManyToManyTables() {
    let em = set_up().await;
    let metadatas = extract_class_metadata(&em, &["CmsUsers", "CmsGroups", "CmsTags"]).await;

    let users = metadata(&metadatas, "CmsUsers");
    let groups = metadata(&metadatas, "CmsGroups");
    let tags = metadata(&metadatas, "CmsTags");

    assert_eq!(3, users.association_mappings.len());
    assert!(users.association_mappings.get("group").is_some());
    assert_eq!(1, groups.association_mappings.len());
    assert!(groups.association_mappings.get("user").is_some());
    assert_eq!(1, tags.association_mappings.len());
    assert!(groups.association_mappings.get("user").is_some());
}

#[tokio::test]
async fn testIgnoreManyToManyTableWithoutFurtherForeignKeyDetails() {
    let em = set_up().await;
    let table_b = table(
        "dbdriver_bar",
        vec![column("id", "integer", &none())],
        &["id"],
        vec![],
    );

    let table_a = table(
        "dbdriver_baz",
        vec![column("id", "integer", &none())],
        &["id"],
        vec![],
    );

    let table_many = table(
        "dbdriver_bar_baz",
        vec![
            column("bar_id", "integer", &none()),
            column("baz_id", "integer", &none()),
        ],
        &[],
        vec![foreign_key(&["bar_id"], "dbdriver_bar", &["id"])],
    );

    let metadatas = convert_to_class_metadata(&em, vec![table_a, table_b], vec![table_many]).await;

    assert_eq!(
        0,
        metadata(&metadatas, "DbdriverBaz")
            .association_mappings
            .len(),
        "no association mappings should be detected."
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines, reason = "upstream's test")]
async fn testLoadMetadataFromDatabaseDetail() {
    let em = set_up().await;
    let string16 = || php_array! { "length" => 16_i64 };
    let mut table = table(
        "dbdriver_foo",
        vec![
            column("id", "integer", &php_array! { "unsigned" => true }),
            column(
                "column_unsigned",
                "integer",
                &php_array! { "unsigned" => true },
            ),
            column(
                "column_comment",
                "string",
                &php_array! { "length" => 16_i64, "comment" => "test_comment" },
            ),
            column(
                "column_default",
                "string",
                &php_array! { "length" => 16_i64, "default" => "test_default" },
            ),
            column(
                "column_decimal",
                "decimal",
                &php_array! { "precision" => 4_i64, "scale" => 3_i64 },
            ),
            column("column_index1", "string", &string16()),
            column("column_index2", "string", &string16()),
            column("column_unique_index1", "string", &string16()),
            column("column_unique_index2", "string", &string16()),
        ],
        &[],
        vec![],
    );

    table
        .add_index(
            ["column_index1", "column_index2"],
            Some("index1"),
            vec![],
            PhpArray::new(),
        )
        .unwrap();
    table
        .add_unique_index(
            ["column_unique_index1", "column_unique_index2"],
            Some("unique_index1"),
            PhpArray::new(),
        )
        .unwrap();

    let table = table
        .edit()
        .unwrap()
        .add_primary_key_constraint(
            php2rust_doctrine_dbal::schema::PrimaryKeyConstraint::new(
                None,
                vec![
                    php2rust_doctrine_dbal::schema::name::UnqualifiedName::unquoted("id").unwrap(),
                ],
                true,
            )
            .unwrap(),
        )
        .create()
        .unwrap();

    drop_and_create_table(&em, &table).await;

    let metadatas = extract_class_metadata(&em, &["DbdriverFoo"]).await;

    let metadata = metadata(&metadatas, "DbdriverFoo");

    let id = metadata.field_mappings.get("id").unwrap();
    assert_eq!("id", id.field_name);
    assert_eq!("id", id.column_name.to_lowercase());
    assert_eq!("integer", id.r#type);

    // SQLite is neither SQL Server nor PostgreSQL: it supports unsigned integers.
    let option = |field: &str, key: &str| -> Value {
        metadata
            .field_mappings
            .get(field)
            .unwrap()
            .options
            .as_ref()
            .unwrap()
            .get(&Key::from(key))
            .cloned()
            .unwrap_or(Value::Null)
    };
    assert_eq!(Value::Bool(true), option("columnUnsigned", "unsigned"));

    assert_eq!(
        Value::string("test_comment"),
        option("columnComment", "comment")
    );

    assert_eq!(
        Value::string("test_default"),
        option("columnDefault", "default")
    );

    let decimal = metadata.field_mappings.get("columnDecimal").unwrap();
    assert_eq!(Some(4), decimal.precision);
    assert_eq!(Some(3), decimal.scale);

    let columns = |constraint_type: &str, name: &str| -> Value {
        let Some(Value::Array(constraints)) = metadata.table.get(&Key::from(constraint_type))
        else {
            panic!("no {constraint_type}");
        };
        let Some(Value::Array(constraint)) = constraints.get(&Key::from(name)) else {
            panic!("no {name}");
        };
        constraint.get(&Key::from("columns")).cloned().unwrap()
    };
    assert_eq!(
        Value::Array(php_array!["column_index1", "column_index2"]),
        columns("indexes", "index1")
    );

    assert_eq!(
        Value::Array(php_array!["column_unique_index1", "column_unique_index2"]),
        columns("uniqueConstraints", "unique_index1")
    );
}
