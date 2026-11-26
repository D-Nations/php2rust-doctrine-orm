//! Ticket/DDC832Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`. `tearDown()` drops the tables; each test here has its own
//! in-memory database, so it is not needed.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC832Test.php

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

use php_value::compare::loose_equals;
use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{get, new_object, s, set};

const DDC832_JOINED_INDEX: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC832JoinedIndex";
const DDC832_JOINED_TREE_INDEX: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC832JoinedTreeIndex";
const DDC832_LIKE: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC832Like";

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[DDC832_JOINED_INDEX, DDC832_JOINED_TREE_INDEX, DDC832_LIKE]).await
}

/// `new DDC832Like($word)`.
fn like(word: &str) -> Object {
    let like = new_object(DDC832_LIKE);
    set(&like, "word", s(word));
    like
}

/// `new DDC832JoinedIndex($name)`.
fn joined_index(name: &str) -> Object {
    let index = new_object(DDC832_JOINED_INDEX);
    set(&index, "name", s(name));
    index
}

/// `new DDC832JoinedTreeIndex($name, $lft, $rgt)`.
fn joined_tree_index(name: &str, lft: i64, rgt: i64) -> Object {
    let index = new_object(DDC832_JOINED_TREE_INDEX);
    set(&index, "lft", Value::Int(lft));
    set(&index, "rgt", Value::Int(rgt));
    set(&index, "name", s(name));
    index
}

/// `self::assertEquals($entity, $this->_em->find($class, $entity->id))`.
async fn assert_found_equals(em: &EntityManager, class: &str, entity: &Object) {
    let found = em
        .find(class, get(entity, "id"))
        .await
        .unwrap()
        .expect("an entity");
    assert!(
        loose_equals(
            &Value::Object(entity.clone()),
            &Value::Object(found.clone())
        ),
        "{entity:?} != {found:?}"
    );
}

async fn assert_not_found(em: &EntityManager, class: &str, id: Value) {
    assert!(em.find(class, id).await.unwrap().is_none());
}

#[tokio::test]
async fn testQuotedTableBasicUpdate() {
    let em = set_up().await;
    let like = like("test");
    em.persist(&like).unwrap();
    em.flush().await.unwrap();

    set(&like, "word", s("test2"));
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_found_equals(&em, DDC832_LIKE, &like).await;
}

#[tokio::test]
async fn testQuotedTableBasicRemove() {
    let em = set_up().await;
    let like = like("test");
    em.persist(&like).unwrap();
    em.flush().await.unwrap();

    let id_to_be_removed = get(&like, "id");

    em.remove(&like).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_not_found(&em, DDC832_LIKE, id_to_be_removed).await;
}

#[tokio::test]
async fn testQuotedTableJoinedUpdate() {
    let em = set_up().await;
    let index = joined_index("test");
    em.persist(&index).unwrap();
    em.flush().await.unwrap();

    set(&index, "name", s("asdf"));
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_found_equals(&em, DDC832_JOINED_INDEX, &index).await;
}

#[tokio::test]
async fn testQuotedTableJoinedRemove() {
    let em = set_up().await;
    let index = joined_index("test");
    em.persist(&index).unwrap();
    em.flush().await.unwrap();

    let id_to_be_removed = get(&index, "id");

    em.remove(&index).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_not_found(&em, DDC832_JOINED_INDEX, id_to_be_removed).await;
}

#[tokio::test]
async fn testQuotedTableJoinedChildUpdate() {
    let em = set_up().await;
    let index = joined_tree_index("test", 1, 2);
    em.persist(&index).unwrap();
    em.flush().await.unwrap();

    set(&index, "name", s("asdf"));
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_found_equals(&em, DDC832_JOINED_TREE_INDEX, &index).await;
}

#[tokio::test]
async fn testQuotedTableJoinedChildRemove() {
    let em = set_up().await;
    let index = joined_tree_index("test", 1, 2);
    em.persist(&index).unwrap();
    em.flush().await.unwrap();

    let id_to_be_removed = get(&index, "id");

    em.remove(&index).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_not_found(&em, DDC832_JOINED_TREE_INDEX, id_to_be_removed).await;
}
