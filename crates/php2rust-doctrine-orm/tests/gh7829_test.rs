//! Ticket/GH7829Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models), with a logging connection (`tests/common/query_log.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7829Test.php

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

#[path = "common/query_log.rs"]
mod query_log;

use std::sync::Arc;

use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Window};

use orm::*;
use query_log::QueryLog;

async fn set_up() -> (Arc<EntityManager>, Arc<QueryLog>) {
    let (em, log) = query_log::set_up_logged(&CMS_MODELS).await;

    let article = cms_article("Skip Limit Subquery", "Skip Limit Subquery if not required.");
    em.persist(&article).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    (em, log)
}

#[tokio::test]
async fn testPaginatorWithLimitSubquery() {
    let (em, log) = set_up().await;
    log.reset_and_enable();

    let query = em.create_query("SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a");

    OffsetPaginator::new(true, Some(false))
        .paginate_window((&query).into(), Window::new(0, 1).unwrap())
        .await
        .unwrap();

    assert_eq!(3, log.queries().len());
}

#[tokio::test]
async fn testPaginatorWithLimitSubquerySkipped() {
    let (em, log) = set_up().await;
    log.reset_and_enable();

    let query = em.create_query("SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a");

    OffsetPaginator::new(false, Some(false))
        .paginate_window((&query).into(), Window::new(0, 1).unwrap())
        .await
        .unwrap();

    assert_eq!(2, log.queries().len());
}
