//! Ticket/DDC2931Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `getRank()` calls the parent's `getRank()`, which initializes a lazy
//! parent: the test initializes it.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2931Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::{HINT_REFRESH, Hint};

use orm::*;
use ticket::*;

/// `$user->getRank()`: `1 + ($this->parent ? $this->parent->getRank() : 0)`.
async fn get_rank(em: &EntityManager, user: &Object) -> i64 {
    let mut rank = 1;
    let mut current = get_object(user, "parent");
    while let Some(parent) = current {
        em.initialize_object(&parent).await.unwrap();
        rank += 1;
        current = get_object(&parent, "parent");
    }
    rank
}

/// Three users, each the parent of the next.
fn users() -> (Object, Object, Object) {
    let first = new("DDC2931User");
    let second = new("DDC2931User");
    let third = new("DDC2931User");

    set_obj(&second, "parent", &first);
    set_obj(&third, "parent", &second);
    (first, second, third)
}

async fn persist_flush(em: &Arc<EntityManager>, users: [&Object; 3]) {
    for user in users {
        em.persist(user).unwrap();
    }
    em.flush().await.unwrap();
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC2931User"]).await;
    let (first, second, third) = users();

    persist_flush(&em, [&first, &second, &third]).await;
    em.clear().unwrap();

    let second = find(&em, "DDC2931User", get(&second, "id")).await.unwrap();

    assert_eq!(2, get_rank(&em, &second).await);
}

#[tokio::test]
async fn testFetchJoinedEntitiesCanBeRefreshed() {
    let em = ticket::set_up(&["DDC2931User"]).await;
    let (first, second, third) = users();

    set(&first, "value", Value::Int(1));
    set(&second, "value", Value::Int(2));
    set(&third, "value", Value::Int(3));

    persist_flush(&em, [&first, &second, &third]).await;

    set(&first, "value", Value::Int(4));
    set(&second, "value", Value::Int(5));
    set(&third, "value", Value::Int(6));

    let query = em.create_query(&format!(
        "SELECT e, p, c FROM {NS}DDC2931User e LEFT JOIN e.parent p LEFT JOIN e.child c WHERE e = :id"
    ));
    query.set_parameter("id", Value::Object(second.clone()), None);
    query.set_hint(HINT_REFRESH, Hint::Value(Value::Bool(true)));
    let refreshed_second = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(1, refreshed_second.len());
    assert_eq!(Value::Int(1), get(&first, "value"));
    assert_eq!(Value::Int(2), get(&second, "value"));
    assert_eq!(Value::Int(3), get(&third, "value"));
}
