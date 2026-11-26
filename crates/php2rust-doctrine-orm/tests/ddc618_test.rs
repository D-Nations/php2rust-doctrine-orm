//! Ticket/DDC618Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! The `UniqueConstraintViolationException` catch of `setUp()` has
//! nothing to catch: every test has a fresh database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC618Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::HYDRATE_ARRAY;

use orm::*;
use ticket::*;

/// `new DDC618Author()`.
fn author() -> Object {
    let author = new("DDC618Author");
    set(
        &author,
        "books",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    author
}

/// `DDC618Author::addBook($title)`.
fn add_book(author: &Object, title: &str) {
    let book = new("DDC618Book");
    set(&book, "title", s(title));
    set_obj(&book, "author", author);
    coll::add(&get(author, "books"), Value::Object(book)).unwrap();
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["DDC618Author", "DDC618Book"]).await;

    // Create author 10/Joe with two books 22/JoeA and 20/JoeB
    let author = author();
    set(&author, "id", Value::Int(10));
    set(&author, "name", s("Joe"));
    em.persist(&author).unwrap();

    // Create author 11/Alice with two books 21/AliceA and 23/AliceB
    let author = self::author();
    set(&author, "id", Value::Int(11));
    set(&author, "name", s("Alice"));
    add_book(&author, "In Wonderland");
    add_book(&author, "Reloaded");
    add_book(&author, "Test");

    em.persist(&author).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

fn has_key(value: &Value, key: impl Into<php_value::Key>) -> bool {
    match value {
        Value::Array(array) => array.get(&key.into()).is_some(),
        _ => false,
    }
}

const AUTHOR_INDEX_BY_NAME: &str = "SELECT A FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC618Author A INDEX BY A.name ORDER BY A.name ASC";

#[tokio::test]
async fn testIndexByHydrateObject() {
    let em = set_up().await;
    let result = em
        .create_query(AUTHOR_INDEX_BY_NAME)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    let _joe = find(&em, "DDC618Author", Value::Int(10)).await;
    let _alice = find(&em, "DDC618Author", Value::Int(11)).await;

    assert!(
        has_key(&result, "Joe"),
        "INDEX BY A.name should return an index by the name of 'Joe'."
    );
    assert!(
        has_key(&result, "Alice"),
        "INDEX BY A.name should return an index by the name of 'Alice'."
    );
}

#[tokio::test]
async fn testIndexByHydrateArray() {
    let em = set_up().await;
    let result = em
        .create_query(AUTHOR_INDEX_BY_NAME)
        .get_result(HYDRATE_ARRAY)
        .await
        .unwrap();

    let _joe = find(&em, "DDC618Author", Value::Int(10)).await;
    let _alice = find(&em, "DDC618Author", Value::Int(11)).await;

    assert!(
        has_key(&result, "Joe"),
        "INDEX BY A.name should return an index by the name of 'Joe'."
    );
    assert!(
        has_key(&result, "Alice"),
        "INDEX BY A.name should return an index by the name of 'Alice'."
    );
}

async fn assert_books_by_title(books: &Value) {
    for title in ["In Wonderland", "Reloaded", "Test"] {
        let present = match books {
            Value::Array(_) => has_key(books, title),
            _ => coll::contains_key(books, title).await.unwrap(),
        };
        assert!(present, "Indexing by title should have books by title.");
    }
}

#[tokio::test]
async fn testIndexByJoin() {
    let em = set_up().await;
    let dql = "SELECT A, B FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC618Author A INNER JOIN A.books B INDEX BY B.title ORDER BY A.name ASC";
    let result = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    let r0 = obj(&at(&result, 0));
    assert_eq!(3, count_of(&r0, "books").await); // Alice, Joe doesn't appear because he has no books.
    assert_eq!(s("Alice"), get(&r0, "name"));
    assert_books_by_title(&get(&r0, "books")).await;

    let result = em
        .create_query(dql)
        .get_result(HYDRATE_ARRAY)
        .await
        .unwrap();

    let r0 = at(&result, 0);
    assert_eq!(3, list(&at(&r0, "books")).len()); // Alice, Joe doesn't appear because he has no books.
    assert_eq!(s("Alice"), at(&r0, "name"));
    assert_books_by_title(&at(&r0, "books")).await;
}

#[tokio::test]
async fn testIndexByToOneJoinSilentlyIgnored() {
    let em = set_up().await;
    let dql = "SELECT B, A FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC618Book B INNER JOIN B.author A INDEX BY A.name ORDER BY A.name ASC";
    let result = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert_instance_of("DDC618Book", &at(&result, 0));
    assert_instance_of("DDC618Author", &get(&obj(&at(&result, 0)), "author"));

    let result = em
        .create_query(dql)
        .get_result(HYDRATE_ARRAY)
        .await
        .unwrap();

    assert_eq!(s("Alice"), at(&at(&at(&result, 0), "author"), "name"));
}

#[tokio::test]
async fn testCombineIndexBy() {
    let em = set_up().await;
    let dql = "SELECT A, B FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC618Author A INDEX BY A.id INNER JOIN A.books B INDEX BY B.title ORDER BY A.name ASC";
    let result = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert!(has_key(&result, 11)); // Alice
    let alice = obj(&at(&result, 11));
    assert_eq!(3, count_of(&alice, "books").await); // Alice, Joe doesn't appear because he has no books.
    assert_eq!(s("Alice"), get(&alice, "name"));
    assert_books_by_title(&get(&alice, "books")).await;
}
