//! Ticket/GH7496WithToIterableTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH7496WithToIterableTest.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::abstract_query::{HYDRATE_ARRAY, HYDRATE_OBJECT};

use orm::*;
use ticket::*;

/// `new $short($id, $name)`.
fn named(short: &str, id: i64, name: &str) -> Object {
    let entity = new(short);
    set(&entity, "id", Value::Int(id));
    set(&entity, "name", s(name));
    entity
}

/// `new GH7496EntityAinB($id, $eA, $eB)`.
fn a_in_b(id: i64, e_a: &Object, e_b: &Object) -> Object {
    let entity = new("GH7496EntityAinB");
    set(&entity, "id", Value::Int(id));
    set_obj(&entity, "eA", e_a);
    set_obj(&entity, "eB", e_b);
    entity
}

#[tokio::test]
async fn testNonUniqueObjectHydrationDuringIteration() {
    let em = ticket::set_up(&["GH7496EntityA", "GH7496EntityB", "GH7496EntityAinB"]).await;
    let a1 = named("GH7496EntityA", 1, "A#1");
    let a2 = named("GH7496EntityA", 2, "A#2");
    let b1 = named("GH7496EntityB", 1, "B#1");
    for entity in [&a1, &a2, &b1, &a_in_b(1, &a1, &b1), &a_in_b(2, &a2, &b1)] {
        em.persist(entity).unwrap();
    }
    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(&format!(
        "SELECT b FROM {} aib JOIN {} b ON aib.eB = b",
        class("GH7496EntityAinB"),
        class("GH7496EntityB")
    ));

    let bs = q
        .to_iterable(None, HYDRATE_OBJECT)
        .await
        .unwrap()
        .to_vec()
        .await
        .unwrap();

    assert_eq!(2, bs.len());
    assert_instance_of("GH7496EntityB", &bs[0]);
    assert_instance_of("GH7496EntityB", &bs[1]);
    assert_loose_eq(&Value::Int(1), &get(&obj(&bs[0]), "id"));
    assert_loose_eq(&Value::Int(1), &get(&obj(&bs[1]), "id"));

    let bs = q
        .to_iterable(None, HYDRATE_ARRAY)
        .await
        .unwrap()
        .to_vec()
        .await
        .unwrap();

    assert_eq!(2, bs.len());
    assert_loose_eq(&Value::Int(1), &at(&bs[0], "id"));
    assert_loose_eq(&Value::Int(1), &at(&bs[1], "id"));
}
