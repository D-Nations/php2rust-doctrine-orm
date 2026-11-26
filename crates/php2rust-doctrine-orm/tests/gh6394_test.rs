//! Ticket/GH6394Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6394Test.php

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

use php_value::Value;

use orm::*;
use ticket::*;

/// Test the version of an entity can be fetched, when the id field and
/// the id column are different.
#[tokio::test]
async fn testFetchVersionValueForDifferentIdFieldAndColumn() {
    let em = ticket::set_up(&["A", "B"]).await;

    // new A(1)
    let a = new("A");
    set(&a, "id", Value::Int(1));
    em.persist(&a).unwrap();

    // new B($a, 'foo')
    let b = new("B");
    set_obj(&b, "a", &a);
    set(&b, "something", s("foo"));
    em.persist(&b).unwrap();
    em.flush().await.unwrap();

    assert!(Value::Int(1).identical(&get(&b, "version")));

    set(&b, "something", s("bar"));
    em.flush().await.unwrap();

    assert!(Value::Int(2).identical(&get(&b, "version")));
}
