//! Ticket/DDC1685Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the `ddc117` model set).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1685Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::Query;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window};

use orm::*;

const DDC117_ARTICLE: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117Article";
const DDC117_ARTICLE_DETAILS: &str = "Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails";

/// The `ddc117` model set.
const DDC117_MODELS: [&str; 7] = [
    DDC117_ARTICLE,
    "Doctrine\\Tests\\Models\\DDC117\\DDC117Reference",
    "Doctrine\\Tests\\Models\\DDC117\\DDC117Translation",
    DDC117_ARTICLE_DETAILS,
    "Doctrine\\Tests\\Models\\DDC117\\DDC117ApproveChanges",
    "Doctrine\\Tests\\Models\\DDC117\\DDC117Editor",
    "Doctrine\\Tests\\Models\\DDC117\\DDC117Link",
];

struct Fixture {
    _em: Arc<EntityManager>,
    query: Query,
    window: Window,
}

async fn set_up() -> Fixture {
    let em = orm::set_up(&DDC117_MODELS).await;

    em.create_query("DELETE FROM Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails ad")
        .execute(None, None)
        .await
        .unwrap();

    // new DDC117Article('Foo')
    let article = new_object(DDC117_ARTICLE);
    set(&article, "title", s("Foo"));
    set(
        &article,
        "references",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(
        &article,
        "translations",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    em.persist(&article).unwrap();
    em.flush().await.unwrap();

    // new DDC117ArticleDetails($article, 'Very long text')
    let article_details = new_object(DDC117_ARTICLE_DETAILS);
    set_obj(&article_details, "article", &article);
    set_obj(&article, "details", &article_details);
    set(&article_details, "text", s("Very long text"));
    em.persist(&article_details).unwrap();
    em.flush().await.unwrap();

    let dql = "SELECT ad FROM Doctrine\\Tests\\Models\\DDC117\\DDC117ArticleDetails ad";
    let query = em.create_query(dql);
    let window = Window::new(0, 1).unwrap();
    Fixture {
        _em: em,
        query,
        window,
    }
}

#[tokio::test]
async fn testPaginateCount() {
    let f = set_up().await;
    let page = OffsetPaginator::new(true, None)
        .paginate_window((&f.query).into(), f.window)
        .await
        .unwrap();
    assert_eq!(1, page.get_total_count());
}

#[tokio::test]
async fn testPaginateIterate() {
    let f = set_up().await;
    let page = OffsetPaginator::new(true, None)
        .paginate_window((&f.query).into(), f.window)
        .await
        .unwrap();
    for ad in page.get_iterator() {
        assert!(matches!(ad, Value::Object(o) if o.instance_of(DDC117_ARTICLE_DETAILS)));
    }
}

#[tokio::test]
async fn testPaginateCountNoOutputWalkers() {
    let f = set_up().await;
    let page = OffsetPaginator::new(false, Some(false))
        .paginate_window((&f.query).into(), f.window)
        .await
        .unwrap();
    assert_eq!(1, page.get_total_count());
}

#[tokio::test]
async fn testPaginateIterateNoOutputWalkers() {
    let f = set_up().await;
    let Err(e) = OffsetPaginator::new(true, Some(false))
        .paginate_window((&f.query).into(), f.window)
        .await
    else {
        panic!("a RuntimeException");
    };
    assert_eq!("RuntimeException", e.class);
    assert_eq!(
        "Paginating an entity with foreign key as identifier only works when using the Output Walkers. Pass $useOutputWalkers = true to the paginator constructor.",
        e.message.to_string_lossy()
    );
}
