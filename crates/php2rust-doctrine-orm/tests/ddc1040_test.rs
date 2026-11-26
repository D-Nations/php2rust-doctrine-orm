//! Ticket/DDC1040Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1040Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&CMS_MODELS).await
}

/// The user, and an article whose author is the user, persisted and flushed.
async fn user_and_article(em: &EntityManager) -> (php_value::Object, php_value::Object) {
    let user = cms_user_with("John Galt", "jgalt", "inactive");

    let article = cms_article("This is John Galt speaking!", "Yadda Yadda!");
    // $article->setAuthor($user)
    set_obj(&article, "user", &user);

    em.persist(&user).unwrap();
    em.persist(&article).unwrap();
    em.flush().await.unwrap();
    (user, article)
}

#[tokio::test]
async fn testReuseNamedEntityParameter() {
    let em = set_up().await;
    let (user, article) = user_and_article(&em).await;

    let dql = "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a WHERE a.user = :author";
    let query = em.create_query(dql);
    query.set_parameter("author", Value::Object(user.clone()), None);
    query.get_result(HYDRATE_OBJECT).await.unwrap();

    let dql = "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a WHERE a.user = :author AND a.user = :author";
    let query = em.create_query(dql);
    query.set_parameter("author", Value::Object(user.clone()), None);
    query.get_result(HYDRATE_OBJECT).await.unwrap();

    let dql = "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a WHERE a.topic = :topic AND a.user = :author AND a.user = :author";
    let query = em.create_query(dql);
    query.set_parameter("author", Value::Object(user), None);
    query.set_parameter("topic", s("This is John Galt speaking!"), None);
    let farticle = query.get_single_result(None).await.unwrap();

    assert!(same_value(&farticle, &article));
}

#[tokio::test]
async fn testUseMultiplePositionalParameters() {
    let em = set_up().await;
    let (user, article) = user_and_article(&em).await;

    let dql = "SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a WHERE a.topic = ?1 AND a.user = ?2 AND a.user = ?3";
    let query = em.create_query(dql);
    query.set_parameter(1, s("This is John Galt speaking!"), None);
    query.set_parameter(2, Value::Object(user.clone()), None);
    query.set_parameter(3, Value::Object(user), None);
    let farticle = query.get_single_result(None).await.unwrap();

    assert!(same_value(&farticle, &article));
}
