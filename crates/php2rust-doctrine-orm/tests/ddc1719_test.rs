//! Ticket/DDC1719Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1719Test.php

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

use php_value::Object;

use orm::*;
use ticket::*;

/// `new DDC1719SimpleEntity($value)`.
fn simple_entity(value: &str) -> Object {
    let entity = new("DDC1719SimpleEntity");
    set(&entity, "value", s(value));
    entity
}

#[tokio::test]
async fn testCreateRetrieveUpdateDelete() {
    let em = ticket::set_up(&["DDC1719SimpleEntity"]).await;

    let e1 = simple_entity("Bar 1");
    let e2 = simple_entity("Foo 1");

    // Create
    em.persist(&e1).unwrap();
    em.persist(&e2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let e1_id = get(&e1, "id");
    let e2_id = get(&e2, "id");

    // Retrieve
    let e1 = find(&em, "DDC1719SimpleEntity", e1_id.clone())
        .await
        .unwrap();
    let e2 = find(&em, "DDC1719SimpleEntity", e2_id.clone())
        .await
        .unwrap();

    assert_eq!(class("DDC1719SimpleEntity"), e1.class());
    assert_eq!(class("DDC1719SimpleEntity"), e2.class());

    assert_loose_eq(&e1_id, &get(&e1, "id"));
    assert_loose_eq(&e2_id, &get(&e2, "id"));

    assert_loose_eq(&s("Bar 1"), &get(&e1, "value"));
    assert_loose_eq(&s("Foo 1"), &get(&e2, "value"));

    set(&e1, "value", s("Bar 2"));
    set(&e2, "value", s("Foo 2"));

    // Update
    em.persist(&e1).unwrap();
    em.persist(&e2).unwrap();
    em.flush().await.unwrap();

    assert_loose_eq(&s("Bar 2"), &get(&e1, "value"));
    assert_loose_eq(&s("Foo 2"), &get(&e2, "value"));

    assert_eq!(class("DDC1719SimpleEntity"), e1.class());
    assert_eq!(class("DDC1719SimpleEntity"), e2.class());

    assert_loose_eq(&e1_id, &get(&e1, "id"));
    assert_loose_eq(&e2_id, &get(&e2, "id"));

    assert_loose_eq(&s("Bar 2"), &get(&e1, "value"));
    assert_loose_eq(&s("Foo 2"), &get(&e2, "value"));

    // Delete
    em.remove(&e1).await.unwrap();
    em.remove(&e2).await.unwrap();
    em.flush().await.unwrap();

    assert!(find(&em, "DDC1719SimpleEntity", e1_id).await.is_none());
    assert!(find(&em, "DDC1719SimpleEntity", e2_id).await.is_none());
}
