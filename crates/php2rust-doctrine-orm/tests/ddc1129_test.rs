//! Ticket/DDC1129Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1129Test.php

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

use php_value::Value;

use orm::*;

#[tokio::test]
async fn testVersionFieldIgnoredInChangesetComputation() {
    let em = orm::set_up(&CMS_MODELS).await;

    let article = cms_article("Who is John Galt?", "I don't know.");

    em.persist(&article).unwrap();
    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(1), &get(&article, "version"));

    let class = em.get_class_metadata(CMS_ARTICLE).unwrap();
    let uow = em.get_unit_of_work();

    uow.compute_change_set(&class, &article).await.unwrap();
    let change_set = uow.get_entity_change_set(&article);
    assert_eq!(0, change_set.len(), "No changesets should be computed.");

    set(&article, "text", s("This is John Galt speaking."));
    em.flush().await.unwrap();

    assert_loose_eq(&Value::Int(2), &get(&article, "version"));

    uow.compute_change_set(&class, &article).await.unwrap();
    let change_set = uow.get_entity_change_set(&article);
    assert_eq!(0, change_set.len(), "No changesets should be computed.");
}
