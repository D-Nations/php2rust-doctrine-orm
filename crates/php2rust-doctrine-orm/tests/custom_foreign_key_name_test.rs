//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CustomForeignKeyNameTest.php
//!
//! On `SQLite` (`tests/common/orm.rs`), with the recorded models of the
//! test file.

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

use std::sync::Arc;

use php_value::Thrown;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::SchemaTool;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\";

fn class(name: &str) -> String {
    format!("{NS}{name}")
}

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[
        &class("CustomFKArticle"),
        &class("CustomFKAuthor"),
        &class("CustomFKTag"),
    ])
    .await
}

/// `createSchemaForModels(...)`: a `ToolsException` is ignored.
async fn create_schema_for_models(em: &Arc<EntityManager>, models: &[String]) -> Result<(), Thrown> {
    let metadata: Vec<_> = models
        .iter()
        .map(|model| em.get_class_metadata(model).unwrap())
        .collect();
    match SchemaTool::new(em).create_schema(&metadata).await {
        Err(e) if e.class == "Doctrine\\ORM\\Tools\\ToolsException" => Ok(()),
        result => result,
    }
}

/// `getSchemaSqlForEntities($entityClasses)`.
fn get_schema_sql_for_entities(em: &Arc<EntityManager>, entity_classes: &[String]) -> Vec<String> {
    let metadata: Vec<_> = entity_classes
        .iter()
        .map(|class| em.get_class_metadata(class).unwrap())
        .collect();
    SchemaTool::new(em).get_create_schema_sql(&metadata).unwrap()
}

/// `/CONSTRAINT\s+(`|")?$name(`|")?\s+FOREIGN\s+KEY/i`.
fn matches_constraint_foreign_key(sql: &str, name: &str) -> bool {
    let lower = sql.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    words.windows(4).any(|w| {
        let quoted = w[1].trim_start_matches(['`', '"']).trim_end_matches(['`', '"']);
        w[0] == "constraint" && quoted == name && w[2] == "foreign" && w[3].starts_with("key")
    })
}

/// `getSchemaSql()`.
fn get_schema_sql(em: &Arc<EntityManager>) -> Vec<String> {
    get_schema_sql_for_entities(
        em,
        &[
            class("CustomFKArticle"),
            class("CustomFKAuthor"),
            class("CustomFKTag"),
        ],
    )
}

#[tokio::test]
async fn testManyToOneGeneratesCustomForeignKeyName() {
    let em = set_up().await;
    let sql = get_schema_sql(&em);
    let sql_string = sql.join("\n");

    // Verify custom FK name is present
    assert!(sql_string.contains("fk_custom_article_author"), "{sql_string}");

    // Verify it's a FOREIGN KEY constraint
    assert!(
        matches_constraint_foreign_key(&sql_string, "fk_custom_article_author"),
        "{sql_string}"
    );
}

#[tokio::test]
async fn testManyToManyGeneratesCustomForeignKeyNames() {
    let em = set_up().await;
    let sql = get_schema_sql(&em);
    let sql_string = sql.join("\n");

    // Verify both FK names
    assert!(sql_string.contains("fk_article_tag_article"), "{sql_string}");
    assert!(sql_string.contains("fk_article_tag_tag"), "{sql_string}");
}

#[tokio::test]
async fn testCompositeForeignKeyWithCustomName() {
    let em = set_up().await;
    let sql = get_schema_sql(&em);
    let sql_string = sql.join("\n");

    // Verify composite FK has custom name
    assert!(sql_string.contains("fk_article_composite_author"), "{sql_string}");
}

#[tokio::test]
async fn testMultipleCompositeFKNamesThrowsException() {
    let em = set_up().await;
    let error = create_schema_for_models(
        &em,
        &[class("InvalidCompositeFKArticle"), class("CustomFKAuthor")],
    )
    .await
    .unwrap_err();

    assert_eq!("RuntimeException", error.class);
    let message = orm::message(&error);
    assert!(message.contains("Only one JoinColumn"), "{message}");
}

#[tokio::test]
async fn testPartialCompositeFKNamesIsValid() {
    let em = set_up().await;
    // This should now be VALID - only one JoinColumn specifies foreignKeyName
    let entities = [class("PartialCompositeFKArticle"), class("CustomFKAuthor")];
    create_schema_for_models(&em, &entities).await.unwrap();

    let sql = get_schema_sql_for_entities(&em, &entities);
    let sql_string = sql.join("\n");

    // Verify custom FK name is present
    assert!(sql_string.contains("fk_partial"), "{sql_string}");
}

#[tokio::test]
async fn testCompositeForeignKeyWithSecondPositionCustomName() {
    let em = set_up().await;
    let entities = [class("SecondPositionCompositeFKArticle"), class("CustomFKAuthor")];
    create_schema_for_models(&em, &entities).await.unwrap();

    let sql = get_schema_sql_for_entities(&em, &entities);
    let sql_string = sql.join("\n");

    // Verify composite FK has custom name
    assert!(sql_string.contains("fk_second_position"), "{sql_string}");
}
