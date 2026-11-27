//! Ticket/DDC3346Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the ddc3346 models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3346Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{append, count_of, new_object, s, set, set_obj};

const AUTHOR: &str = "Doctrine\\Tests\\Models\\DDC3346\\DDC3346Author";
const ARTICLE: &str = "Doctrine\\Tests\\Models\\DDC3346\\DDC3346Article";

/// `setUp()` with `useModelSet('ddc3346')` and `loadAuthorFixture()`.
async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&[AUTHOR, ARTICLE]).await;

    let user = new_object(AUTHOR);
    let article1 = new_object(ARTICLE);
    let article2 = new_object(ARTICLE);

    set(&user, "username", s("bwoogy"));
    set_obj(&article1, "user", &user);
    set_obj(&article2, "user", &user);
    append(&user, "articles", Value::Object(article1.clone()));
    append(&user, "articles", Value::Object(article2.clone()));

    em.persist(&user).unwrap();
    em.persist(&article1).unwrap();
    em.persist(&article2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

fn by_username() -> PhpArray {
    let mut criteria = PhpArray::new();
    criteria.insert("username", s("bwoogy"));
    criteria
}

#[tokio::test]
async fn testFindOneWithEagerFetchWillNotHydrateLimitedCollection() {
    let em = set_up().await;
    let author = em
        .get_repository(AUTHOR)
        .unwrap()
        .find_one_by(&by_username(), None)
        .await
        .unwrap()
        .unwrap();
    assert!(author.instance_of(AUTHOR));

    assert_eq!(2, count_of(&author, "articles").await);
}

#[tokio::test]
async fn testFindLimitedWithEagerFetchWillNotHydrateLimitedCollection() {
    let em = set_up().await;
    let authors = em
        .get_repository(AUTHOR)
        .unwrap()
        .find_by(&by_username(), None, Some(1), None)
        .await
        .unwrap();

    assert_eq!(1, authors.len());
    assert_eq!(2, count_of(&authors[0], "articles").await);
}

#[tokio::test]
async fn testFindWithEagerFetchAndOffsetWillNotHydrateLimitedCollection() {
    let em = set_up().await;
    let authors = em
        .get_repository(AUTHOR)
        .unwrap()
        // using an explicitly defined offset
        .find_by(&by_username(), None, None, Some(0))
        .await
        .unwrap();

    assert_eq!(1, authors.len());
    assert_eq!(2, count_of(&authors[0], "articles").await);
}
