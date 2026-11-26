//! Ticket/GH12183Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12183Test.php

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

use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window};

use orm::*;

#[tokio::test]
async fn testPaginatorCountWithTreeWalkerAfterQueryHasBeenExecuted() {
    // setUp()
    let em = orm::set_up(&CMS_MODELS).await;
    let article = cms_article("Loomings", "Call me Ishmael.");
    em.persist(&article).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query("SELECT a FROM Doctrine\\Tests\\Models\\CMS\\CmsArticle a");

    let paginator = OffsetPaginator::new(true, Some(false));
    let window = Window::new(0, 10).unwrap();

    // the total count is right when the query has not yet been executed
    assert_eq!(
        1,
        paginator
            .paginate_window((&query).into(), window)
            .await
            .unwrap()
            .get_total_count()
    );

    // Execute the query
    let result = list(&query.get_object_result().await.unwrap());
    assert_eq!(1, result.len());

    assert_eq!(
        1,
        paginator
            .paginate_window((&query).into(), window)
            .await
            .unwrap()
            .get_total_count()
    );
}
