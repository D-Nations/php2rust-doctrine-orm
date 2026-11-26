//! Ticket/DDC309Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `toIterable()` yields the hydrated rows of a statement read whole:
//! the two iterations interleave as PHP's generators do.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC309Test.php

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
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testTwoToIterableHydrations() {
    let em = ticket::set_up(&["DDC309Country", "DDC309User"]).await;

    let c1 = new("DDC309Country");
    let c2 = new("DDC309Country");
    let u1 = new("DDC309User");
    let u2 = new("DDC309User");

    em.persist(&c1).unwrap();
    em.persist(&c2).unwrap();
    em.persist(&u1).unwrap();
    em.persist(&u2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let mut q = em
        .create_query("SELECT c FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC309Country c")
        .to_iterable(None, HYDRATE_OBJECT)
        .await
        .unwrap()
        .into_iter();
    let c = q.next().unwrap();
    assert_loose_eq(&Value::Int(1), &get(&obj(&c), "id"));

    let mut r = em
        .create_query("SELECT u FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC309User u")
        .to_iterable(None, HYDRATE_OBJECT)
        .await
        .unwrap()
        .into_iter();
    let u = r.next().unwrap();
    assert_loose_eq(&Value::Int(1), &get(&obj(&u), "id"));

    let c = q.next().unwrap();
    let u = r.next().unwrap();

    assert_loose_eq(&Value::Int(2), &get(&obj(&c), "id"));
    assert_loose_eq(&Value::Int(2), &get(&obj(&u), "id"));

    for _ in q.by_ref() {}
    for _ in r.by_ref() {}
}
