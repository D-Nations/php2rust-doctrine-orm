//! Ticket/DDC812Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC812Test.php

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

use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;

#[tokio::test]
async fn testFetchJoinInitializesPreviouslyUninitializedCollectionOfManagedEntity() {
    let em = orm::set_up(&CMS_MODELS).await;

    let article = cms_article("hello", "talk talk talk");

    let comment = cms_comment("good!", "stuff!");
    set_obj(&comment, "article", &article);

    em.persist(&article).unwrap();
    em.persist(&comment).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let article2 = em
        .find(CMS_ARTICLE, get(&article, "id"))
        .await
        .unwrap()
        .unwrap();

    let query = em.create_query(
        "select a, c from Doctrine\\Tests\\Models\\CMS\\CmsArticle a join a.comments c where a.id = ?1",
    );
    query.set_parameter(1, get(&article, "id"), None);
    let article2_again = obj(&query.get_single_result(None).await.unwrap());

    assert!(same(&article2_again, &article2));
    assert!(
        PersistentCollection::from_value(&get(&article2_again, "comments"))
            .unwrap()
            .is_initialized()
    );
}
