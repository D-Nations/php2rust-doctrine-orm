//! Ticket/GH7875Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7875Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_dbal::{Configuration, DriverManager, ParameterTypes};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::SchemaTool;

use ticket::*;

/// `getUpdateSchemaSqlForModels(...$classes)`.
async fn get_update_schema_sql_for_models(em: &Arc<EntityManager>, shorts: &[&str]) -> Vec<String> {
    let classes: Vec<_> = shorts
        .iter()
        .map(|short| em.get_class_metadata(&class(short)).unwrap())
        .collect();
    SchemaTool::new(em)
        .get_update_schema_sql(&classes)
        .await
        .unwrap()
}

/// `filterCreateTable($sqls, $tableName)`.
fn filter_create_table(sqls: &[String], table_name: &str) -> Vec<String> {
    let prefix = format!("CREATE TABLE {table_name} (");
    sqls.iter()
        .filter(|sql| sql.starts_with(&prefix))
        .cloned()
        .collect()
}

async fn execute_statement(em: &EntityManager, sql: &str) {
    em.get_connection()
        .execute_statement(sql, &PhpArray::new(), &ParameterTypes::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn testUpdateSchemaSql() {
    let em = orm::entity_manager(orm::connection(), None);
    let mut classes = vec!["GH7875MyEntity"];

    let sqls = filter_create_table(
        &get_update_schema_sql_for_models(&em, &classes).await,
        "gh7875_my_entity",
    );
    assert_eq!(1, sqls.len());

    execute_statement(&em, &sqls[0]).await;

    let sqls: Vec<String> = get_update_schema_sql_for_models(&em, &classes)
        .await
        .into_iter()
        .filter(|sql| sql.contains(" gh7875_my_entity "))
        .collect();
    assert_eq!(Vec::<String>::new(), sqls);

    classes.push("GH7875MyOtherEntity");

    let sqls = get_update_schema_sql_for_models(&em, &classes).await;

    assert_eq!(0, filter_create_table(&sqls, "gh7875_my_entity").len());
    assert_eq!(
        1,
        filter_create_table(&sqls, "gh7875_my_other_entity").len()
    );
}

/// The connection's configuration cannot change once it is built, so the
/// filter is set when the connection is made (`createSchemaForModels()`
/// does not read it).
#[tokio::test]
async fn testUpdateSchemaSqlWithSchemaAssetFilter() {
    let mut config = Configuration::new();
    config.set_schema_assets_filter(Arc::new(|asset_name: &str| {
        asset_name != "gh7875_my_entity"
    }));
    let mut params = PhpArray::new();
    params.insert("driver", Value::string("pdo_sqlite"));
    params.insert("memory", Value::Bool(true));
    let conn = Arc::new(DriverManager::get_connection(params, Some(config)).unwrap());
    let em = orm::entity_manager(conn, None);

    let class = "GH7875MyEntity";
    orm::create_schema(&em, &[&ticket::class(class)]).await;

    let previous_filter = em
        .get_connection()
        .get_configuration()
        .get_schema_assets_filter()
        .clone();

    let sqls: Vec<String> = get_update_schema_sql_for_models(&em, &[class])
        .await
        .into_iter()
        .filter(|sql| sql.contains(" gh7875_my_entity "))
        .collect();

    assert_eq!(0, sqls.len());
    assert!(Arc::ptr_eq(
        &previous_filter,
        em.get_connection()
            .get_configuration()
            .get_schema_assets_filter()
    ));
}
