//! CursorPaginatorTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on a driver connection that gives the rows queued for each query
//! (`tests/common/recording_connection.rs`). Upstream stubs the hydrator
//! with `stdClass` items (`(object) ['id' => 1]`): here the real object
//! hydrator builds `BlogPost` entities from rows with those identifiers
//! (and `Author` references for `testGetNextCursorUnwrapsManyToOneAssociationToIdentifier`),
//! and the COUNT query reads a `sclr_0` row. `executeQuery()`'s expected
//! call count is the count of recorded queries. The page's count and
//! cursor getters are `async`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/CursorPaginatorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/recording_connection.rs"]
mod recording_connection;

use std::sync::Arc;

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_orm::QueryBuilder;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::Query;
use php2rust_doctrine_orm::query::SortDirection;
use php2rust_doctrine_orm::query::context::{Hint, QueryContext};
use php2rust_doctrine_orm::tools::pagination::{
    Cursor, CursorPage, CursorPaginator, InvalidCursor, PaginatorInterface,
};

use recording_connection::{SharedRecorder, recording_connection, row};

const BLOG_POST_DQL: &str =
    r"SELECT p FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost p ORDER BY p.id ASC";

struct SetUp {
    em: Arc<EntityManager>,
    recorder: SharedRecorder,
}

fn set_up() -> SetUp {
    models::register_query_models();
    let (connection, recorder) = recording_connection();
    SetUp {
        em: orm::entity_manager(connection, None),
        recorder,
    }
}

/// A `BlogPost` row of the page query.
fn post(id: i64, author_id: Option<i64>) -> PhpArray {
    let mut row = PhpArray::new();
    row.insert("id_0", Value::Int(id));
    row.insert("author_id_1", author_id.map_or(Value::Null, Value::Int));
    row.insert("category_id_2", Value::Null);
    row
}

impl SetUp {
    /// `$this->hydrator->method('hydrateAll')->willReturn(...)`: the rows
    /// of the next query.
    fn rows(&self, rows: Vec<PhpArray>) {
        self.recorder.lock().unwrap().rows.push_back(rows);
    }

    fn posts(&self, ids: &[i64]) {
        self.rows(ids.iter().map(|id| post(*id, None)).collect());
    }

    fn query(&self, dql: &str) -> Query {
        let query = Query::new(&self.em);
        query.set_dql(dql);
        query
    }

    fn query_count(&self) -> usize {
        self.recorder.lock().unwrap().received_params.len()
    }
}

async fn paginate(
    paginator: CursorPaginator,
    query: &Query,
    position: &Value,
) -> Result<CursorPage, Thrown> {
    paginator.paginate(query.into(), position).await
}

fn without_duplicates(limit: i64) -> CursorPaginator {
    CursorPaginator::new(limit, false, None)
}

fn id(item: &Value) -> Value {
    orm::get(item.as_object().unwrap(), "id")
}

fn ids(page: &CursorPage) -> Vec<Value> {
    page.get_items().iter().map(id).collect()
}

#[tokio::test]
async fn testPaginatorAcceptsQueryBuilder() {
    let t = set_up();
    t.posts(&[1]);

    // not $this->em->createQueryBuilder(): the decorator would bind the builder to the
    // wrapped entity manager, bypassing the mocked hydrator.
    let mut qb = QueryBuilder::new(Arc::clone(&t.em) as Arc<dyn QueryContext>);
    qb.select(vec![Value::from("p")])
        .unwrap()
        .from(r"Doctrine\Tests\ORM\Tools\Pagination\BlogPost", "p", None)
        .unwrap()
        .order_by("p.id", Some(SortDirection::Ascending.into()))
        .unwrap();

    let page = without_duplicates(10)
        .paginate((&qb).into(), &Value::Null)
        .await
        .unwrap();

    assert_eq!(vec![Value::Int(1)], ids(&page));
}

#[tokio::test]
async fn testPaginatorIsReusableAcrossQueriesAndPages() {
    let t = set_up();
    t.posts(&[1]);
    t.posts(&[1]);

    let query = t.query(BLOG_POST_DQL);

    let other_query =
        t.query(r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b ORDER BY b.id ASC");

    let paginator = without_duplicates(10);

    assert_eq!(
        1,
        paginate(paginator, &query, &Value::Null)
            .await
            .unwrap()
            .count()
    );
    assert_eq!(
        1,
        paginate(paginator, &other_query, &Value::Null)
            .await
            .unwrap()
            .count()
    );
}

#[tokio::test]
async fn testHasNextPageWhenMoreResultsExist() {
    let t = set_up();
    t.posts(&[1, 2, 3, 4]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(3), &query, &Value::Null)
        .await
        .unwrap();

    assert!(page.has_next_page());
    assert!(!page.has_previous_page());
    assert_eq!(3, page.count());
}

#[tokio::test]
async fn testHasNoPagesOnFirstPageWithoutMoreResults() {
    let t = set_up();
    t.posts(&[1]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();

    assert!(!page.has_previous_page());
    assert!(!page.has_next_page());
    assert!(!page.has_to_paginate());
}

async fn single_item_page(t: &SetUp) -> CursorPage {
    t.posts(&[1]);

    let query = t.query(BLOG_POST_DQL);

    paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap()
}

#[tokio::test]
async fn testGetNextCursorAsStringThrowsWhenNoNextPage() {
    let t = set_up();
    let page = single_item_page(&t).await;

    let e = page.get_next_cursor_as_string().await.unwrap_err();
    assert_eq!("LogicException", e.class);
}

#[tokio::test]
async fn testGetPreviousCursorAsStringThrowsWhenNoPreviousPage() {
    let t = set_up();
    let page = single_item_page(&t).await;

    let e = page.get_previous_cursor_as_string().await.unwrap_err();
    assert_eq!("LogicException", e.class);
}

#[tokio::test]
async fn testGetNextCursorThrowsWhenNoNextPage() {
    let t = set_up();
    let page = single_item_page(&t).await;

    let e = page.get_next_cursor().await.unwrap_err();
    assert_eq!("LogicException", e.class);
}

#[tokio::test]
async fn testGetPreviousCursorThrowsWhenNoPreviousPage() {
    let t = set_up();
    let page = single_item_page(&t).await;

    let e = page.get_previous_cursor().await.unwrap_err();
    assert_eq!("LogicException", e.class);
}

#[tokio::test]
async fn testGetNextCursorWhenMoreResultsExist() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(1), &query, &Value::Null)
        .await
        .unwrap();

    let next_cursor = page.get_next_cursor().await.unwrap();

    assert!(next_cursor.is_next());
}

#[tokio::test]
async fn testGetNextCursorAsStringReturnsStringWhenNextPageExists() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(1), &query, &Value::Null)
        .await
        .unwrap();

    assert!(!page.get_next_cursor_as_string().await.unwrap().is_empty());
}

fn cursor(key: &str, value: i64, is_next: bool) -> Cursor {
    let mut parameters = PhpArray::new();
    parameters.insert(key, Value::Int(value));
    Cursor::new(parameters, is_next)
}

#[tokio::test]
async fn testGetPreviousCursorAsStringReturnsStringWhenPreviousPageExists() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let cursor = cursor("p__id", 1, true).encode_to_string();
    let page = paginate(without_duplicates(10), &query, &Value::from(cursor))
        .await
        .unwrap();

    assert!(
        !page
            .get_previous_cursor_as_string()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn testEmptyResultSet() {
    let t = set_up();
    t.posts(&[]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();

    assert_eq!(0, page.count());
    assert!(!page.has_next_page());
    assert!(!page.has_previous_page());
}

#[tokio::test]
async fn testIteratorReturnsItems() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();

    let mut iterated_items = Vec::new();
    for item in page.get_iterator() {
        iterated_items.push(item.clone());
    }

    assert_eq!(2, iterated_items.len());
}

#[tokio::test]
async fn testGetItemsReturnsRawEntities() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();

    let values = page.get_items();

    assert_eq!(2, values.len());
    assert_eq!(Value::Int(1), id(&values[0]));
    assert_eq!(Value::Int(2), id(&values[1]));
}

#[tokio::test]
async fn testGetItemsWithCursorsReturnsCursorItems() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();
    let items = page.get_items();

    let cursor_items = page.get_items_with_cursors().await.unwrap();

    assert_eq!(2, cursor_items.len());
    assert!(items[0].identical(cursor_items[0].get_value()));
    assert!(items[1].identical(cursor_items[1].get_value()));
    assert_eq!(
        Some(&Value::Int(1)),
        cursor_items[0]
            .get_cursor()
            .get_parameters()
            .get(&"p.id".into())
    );
}

#[tokio::test]
async fn testGetItemsReturnsEmptyArrayWhenNoResults() {
    let t = set_up();
    t.posts(&[]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();

    assert!(page.get_items_with_cursors().await.unwrap().is_empty());
    assert!(page.get_items().is_empty());
}

#[tokio::test]
async fn testPaginateWithCursorInstance() {
    let t = set_up();
    t.posts(&[1, 2]);

    let query = t.query(BLOG_POST_DQL);

    let cursor = cursor("p.id", 1, true);
    let page = paginate(without_duplicates(10), &query, &Value::from(cursor))
        .await
        .unwrap();

    assert!(
        !page
            .get_previous_cursor_as_string()
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn testPaginateWithEmptyCursorTreatedAsFirstPage() {
    let t = set_up();
    t.posts(&[1]);

    let query = t.query(BLOG_POST_DQL);

    let page = paginate(without_duplicates(10), &query, &Value::from(""))
        .await
        .unwrap();

    assert!(!page.has_previous_page());
    assert!(!page.has_next_page());
}

#[tokio::test]
async fn testPaginateWithPreviousCursorReversesItems() {
    let t = set_up();
    t.posts(&[3, 2, 1]);

    let query = t.query(BLOG_POST_DQL);

    let previous_cursor = cursor("p__id", 3, false).encode_to_string();

    let page = paginate(
        without_duplicates(10),
        &query,
        &Value::from(previous_cursor),
    )
    .await
    .unwrap();

    assert_eq!(
        vec![Value::Int(1), Value::Int(2), Value::Int(3)],
        ids(&page)
    );
}

#[tokio::test]
async fn testPaginateThrowsInvalidCursorForNonStringNonCursorNonNull() {
    let t = set_up();
    let query = t.query(BLOG_POST_DQL);

    let mut position = PhpArray::new();
    position.insert("injected", Value::from("array"));
    let e = paginate(without_duplicates(10), &query, &Value::Array(position))
        .await
        .unwrap_err();
    assert_eq!(InvalidCursor::CLASS, e.class);
}

#[test]
fn testDefaultQueryProducesDuplicatesIsTrue() {
    let paginator = CursorPaginator::new(10, true, None);

    assert_eq!(10, paginator.get_limit());
    assert!(paginator.get_query_produces_duplicates());
}

#[tokio::test]
async fn testGetTotalCountReturnsCountQueryResult() {
    let t = set_up();
    t.rows(Vec::new());
    t.rows(vec![row("sclr_0", 42)]);

    let query = t.query(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.id ASC");

    let page = paginate(
        CursorPaginator::new(10, false, Some(false)),
        &query,
        &Value::Null,
    )
    .await
    .unwrap();

    assert_eq!(42, page.get_total_count().await.unwrap());
}

#[tokio::test]
async fn testGetTotalCountIsCached() {
    let t = set_up();
    t.rows(Vec::new());
    t.rows(vec![row("sclr_0", 5)]);

    let query = t.query(r"SELECT u FROM Doctrine\Tests\Models\CMS\CmsUser u ORDER BY u.id ASC");

    let page = paginate(
        CursorPaginator::new(10, false, Some(false)),
        &query,
        &Value::Null,
    )
    .await
    .unwrap();

    assert_eq!(5, page.get_total_count().await.unwrap());
    assert_eq!(5, page.get_total_count().await.unwrap());
    // one executeQuery for the page query, one for the (cached) count query
    assert_eq!(2, t.query_count());
}

#[tokio::test]
async fn testCloneQueryCopiesExistingHints() {
    let t = set_up();
    t.posts(&[1]);

    let query = t.query(BLOG_POST_DQL);
    query.set_hint("custom.hint", Hint::Value(Value::from("custom_value")));

    let page = paginate(without_duplicates(10), &query, &Value::Null)
        .await
        .unwrap();

    assert_eq!(1, page.count());
}

#[tokio::test]
async fn testGetNextCursorUnwrapsManyToOneAssociationToIdentifier() {
    let t = set_up();
    t.rows(vec![post(1, Some(42)), post(2, Some(43))]);

    let query = t.query(
        r"SELECT b FROM Doctrine\Tests\ORM\Tools\Pagination\BlogPost b ORDER BY b.author ASC",
    );

    let page = paginate(without_duplicates(1), &query, &Value::Null)
        .await
        .unwrap();

    assert!(page.has_next_page());

    let cursor = page.get_next_cursor().await.unwrap();
    assert_eq!(
        Some(&Value::Int(42)),
        cursor.get_parameters().get(&"b.author".into())
    );
}
