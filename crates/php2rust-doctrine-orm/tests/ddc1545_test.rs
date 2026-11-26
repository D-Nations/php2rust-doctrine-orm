//! Ticket/DDC1545Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1545Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

struct T {
    em: Arc<EntityManager>,
    article_id: Value,
    user_id: Value,
    user2_id: Value,
}

async fn init_db(link: bool) -> T {
    let em = orm::set_up(&CMS_MODELS).await;

    let article = cms_article("foo", "foo");

    let user = cms_user_with("foo", "foo", "foo");

    let user2 = cms_user_with("bar", "bar", "bar");

    if link {
        set_obj(&article, "user", &user);
    }

    em.persist(&article).unwrap();
    em.persist(&user).unwrap();
    em.persist(&user2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    T {
        em,
        article_id: get(&article, "id"),
        user_id: get(&user, "id"),
        user2_id: get(&user2, "id"),
    }
}

impl T {
    async fn find(&self, class: &str, id: &Value) -> Object {
        self.em.find(class, id.clone()).await.unwrap().unwrap()
    }

    /// The article, with its user joined.
    async fn query_article(&self) -> Object {
        let query = self.em.create_query(
            "SELECT a, u FROM Doctrine\\Tests\\Models\\Cms\\CmsArticle a LEFT JOIN a.user u WHERE a.id = :id",
        );
        query.set_parameter("id", self.article_id.clone(), None);
        obj(&query.get_one_or_null_result(None).await.unwrap())
    }

    async fn flush_clear(&self) {
        self.em.flush().await.unwrap();
        self.em.clear().unwrap();
    }
}

#[tokio::test]
async fn testLinkObjects() {
    let t = init_db(false).await;

    // don't join association
    let article = t.find(CMS_ARTICLE, &t.article_id).await;

    let user = t.find(CMS_USER, &t.user_id).await;

    set_obj(&article, "user", &user);

    t.flush_clear().await;

    let article = t.query_article().await;

    let article_user = get_object(&article, "user").expect("assertNotNull");
    assert_loose_eq(&get(&user, "id"), &get(&article_user, "id"));
}

#[tokio::test]
async fn testLinkObjectsWithAssociationLoaded() {
    let t = init_db(false).await;

    // join association
    let article = t.query_article().await;

    let user = t.find(CMS_USER, &t.user_id).await;

    set_obj(&article, "user", &user);

    t.flush_clear().await;

    let article = t.query_article().await;

    let article_user = get_object(&article, "user").expect("assertNotNull");
    assert_loose_eq(&get(&user, "id"), &get(&article_user, "id"));
}

#[tokio::test]
async fn testUnlinkObjects() {
    let t = init_db(true).await;

    // don't join association
    let article = t.find(CMS_ARTICLE, &t.article_id).await;

    set(&article, "user", Value::Null);

    t.flush_clear().await;

    let article = t.query_article().await;

    assert_eq!(Value::Null, get(&article, "user"));
}

#[tokio::test]
async fn testUnlinkObjectsWithAssociationLoaded() {
    let t = init_db(true).await;

    // join association
    let article = t.query_article().await;

    set(&article, "user", Value::Null);

    t.flush_clear().await;

    let article = t.query_article().await;

    assert_eq!(Value::Null, get(&article, "user"));
}

#[tokio::test]
async fn testChangeLink() {
    let t = init_db(false).await;

    // don't join association
    let article = t.find(CMS_ARTICLE, &t.article_id).await;

    let user2 = t.find(CMS_USER, &t.user2_id).await;

    set_obj(&article, "user", &user2);

    t.flush_clear().await;

    let article = t.query_article().await;

    let article_user = get_object(&article, "user").expect("assertNotNull");
    assert_loose_eq(&get(&user2, "id"), &get(&article_user, "id"));
}

#[tokio::test]
async fn testChangeLinkWithAssociationLoaded() {
    let t = init_db(false).await;

    // join association
    let article = t.query_article().await;

    let user2 = t.find(CMS_USER, &t.user2_id).await;

    set_obj(&article, "user", &user2);

    t.flush_clear().await;

    let article = t.query_article().await;

    let article_user = get_object(&article, "user").expect("assertNotNull");
    assert_loose_eq(&get(&user2, "id"), &get(&article_user, "id"));
}
