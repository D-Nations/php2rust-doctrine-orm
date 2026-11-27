//! Port of `ReadonlyPropertiesTest`, on `SQLite`. The attribute driver
//! reads the recorded `Models/ReadonlyProperties` classes.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ReadonlyPropertiesTest.php

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

use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{assert_loose_eq, elements, get, get_object, list, obj, s};

const NS: &str = "Doctrine\\Tests\\Models\\ReadonlyProperties\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[&class("Author"), &class("Book"), &class("SimpleBook")]).await
}

/// `$connection->insert($table, $data)` and `lastInsertId()`.
async fn insert(em: &EntityManager, table: &str, data: PhpArray) -> Value {
    let connection = em.get_connection();
    connection
        .insert(table, &data, &ParameterTypes::new())
        .await
        .unwrap();
    connection.last_insert_id().await.unwrap()
}

#[tokio::test]
async fn testSimpleEntity() {
    let em = set_up().await;
    let author_id = insert(&em, "author", php_array! { "name" => "Jane Austen" }).await;

    let author = em
        .find(&class("Author"), author_id.clone())
        .await
        .unwrap()
        .unwrap();

    assert!(s("Jane Austen").identical(&get(&author, "name")));
    assert_loose_eq(&author_id, &get(&author, "id"));
}

#[tokio::test]
async fn testEntityWithLazyManyToOne() {
    let em = set_up().await;
    let author_id = insert(&em, "author", php_array! { "name" => "Jane Austen" }).await;

    let mut book = php_array! { "title" => "Pride and Prejudice" };
    book.insert("author_id", author_id);
    let book_id = insert(&em, "simple_book", book).await;

    let book = em
        .find(&class("SimpleBook"), book_id.clone())
        .await
        .unwrap()
        .unwrap();

    assert!(s("Pride and Prejudice").identical(&get(&book, "title")));
    assert_loose_eq(&book_id, &get(&book, "id"));
    let author = get_object(&book, "author").unwrap();
    em.initialize_object(&author).await.unwrap();
    assert!(s("Jane Austen").identical(&get(&author, "name")));
}

#[tokio::test]
async fn testEntityWithEagerManyToOne() {
    let em = set_up().await;
    let author_id = insert(&em, "author", php_array! { "name" => "Jane Austen" }).await;

    let mut book = php_array! { "title" => "Pride and Prejudice" };
    book.insert("author_id", author_id);
    let book_id = insert(&em, "simple_book", book).await;

    let mut qb = em.create_query_builder();
    qb.from(&class("SimpleBook"), "b", None)
        .unwrap()
        .join("b.author", "a", None, None, None)
        .unwrap()
        .select(vec![s("b"), s("a")])
        .unwrap()
        .r#where(vec![s("b.id = :id")])
        .unwrap()
        .set_parameter("id", book_id.clone(), None);
    let result = qb.get_query().unwrap().execute(None, None).await.unwrap();
    let book = obj(&list(&result)[0]);

    assert_eq!(book.class(), class("SimpleBook"));
    assert!(s("Pride and Prejudice").identical(&get(&book, "title")));
    assert_loose_eq(&book_id, &get(&book, "id"));
    let author = get_object(&book, "author").unwrap();
    assert!(s("Jane Austen").identical(&get(&author, "name")));
}

#[tokio::test]
async fn testEntityWithManyToMany() {
    let em = set_up().await;
    let author_id = insert(&em, "author", php_array! { "name" => "Jane Austen" }).await;

    let book_id = insert(&em, "book", php_array! { "title" => "Pride and Prejudice" }).await;

    let mut book_author = PhpArray::new();
    book_author.insert("book_id", book_id.clone());
    book_author.insert("author_id", author_id);
    insert(&em, "book_author", book_author).await;

    let book = em
        .find(&class("Book"), book_id.clone())
        .await
        .unwrap()
        .unwrap();

    assert!(s("Pride and Prejudice").identical(&get(&book, "title")));
    assert_loose_eq(&book_id, &get(&book, "id"));
    let authors = elements(&book, "authors").await;
    assert!(s("Jane Austen").identical(&get(&authors[0], "name")));
}
