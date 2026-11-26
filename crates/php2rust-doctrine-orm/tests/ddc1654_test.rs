//! Ticket/DDC1654Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `tearDown()`'s DELETE statements are left out: every test has a
//! fresh database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1654Test.php

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
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// A persisted post with two comments.
async fn set_up() -> (Arc<EntityManager>, Object) {
    let em = ticket::set_up(&["DDC1654Post", "DDC1654Comment"]).await;

    let post = new("DDC1654Post");
    set(&post, "comments", Value::Array(PhpArray::new()));
    append(&post, "comments", Value::Object(new("DDC1654Comment")));
    append(&post, "comments", Value::Object(new("DDC1654Comment")));

    em.persist(&post).unwrap();
    em.flush().await.unwrap();
    (em, post)
}

async fn comment_count(em: &Arc<EntityManager>) -> usize {
    em.clear().unwrap();
    find_all(em, "DDC1654Comment").await.len()
}

#[tokio::test]
async fn testManyToManyRemoveFromCollectionOrphanRemoval() {
    let (em, post) = set_up().await;
    let comments = get(&post, "comments");

    coll::remove(&comments, 0).await.unwrap();
    coll::remove(&comments, 1).await.unwrap();
    em.flush().await.unwrap();

    assert_eq!(0, comment_count(&em).await);
}

#[tokio::test]
async fn testManyToManyRemoveElementFromCollectionOrphanRemoval() {
    let (em, post) = set_up().await;
    let comments = get(&post, "comments");

    let first = coll::get(&comments, 0).await.unwrap();
    coll::remove_element(&comments, &first).await.unwrap();
    let second = coll::get(&comments, 1).await.unwrap();
    coll::remove_element(&comments, &second).await.unwrap();
    em.flush().await.unwrap();

    assert_eq!(0, comment_count(&em).await);
}

#[tokio::test]
async fn testManyToManyRemoveElementFromReAddToCollectionOrphanRemoval() {
    let (em, post) = set_up().await;
    let comments = get(&post, "comments");

    let comment = coll::get(&comments, 0).await.unwrap();
    coll::remove_element(&comments, &comment).await.unwrap();
    coll::add(&comments, comment).unwrap();
    em.flush().await.unwrap();

    assert_eq!(2, comment_count(&em).await);
}

#[tokio::test]
async fn testManyToManyClearCollectionOrphanRemoval() {
    let (em, post) = set_up().await;
    let comments = get(&post, "comments");

    coll::clear(&comments).await.unwrap();
    em.flush().await.unwrap();

    assert_eq!(0, comment_count(&em).await);
}

#[tokio::test]
async fn testManyToManyClearCollectionReAddOrphanRemoval() {
    let (em, post) = set_up().await;
    let comments = get(&post, "comments");

    let comment = coll::get(&comments, 0).await.unwrap();
    coll::clear(&comments).await.unwrap();
    coll::add(&comments, comment).unwrap();
    em.flush().await.unwrap();

    assert_eq!(1, comment_count(&em).await);
}
