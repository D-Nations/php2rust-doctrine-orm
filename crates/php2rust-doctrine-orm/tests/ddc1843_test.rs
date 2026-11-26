//! Ticket/DDC1843Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the quote models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1843Test.php

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
#[path = "common/quote.rs"]
mod quote;

use orm::*;
use quote::*;

#[tokio::test]
async fn testCreateRetrieveUpdateDelete() {
    let em = orm::set_up(&QUOTE_MODELS).await;

    let e1 = quote_group("Parent Bar 1", None);
    let e2 = quote_group("Parent Foo 2", None);

    em.persist(&e1).unwrap();
    em.persist(&e2).unwrap();
    em.flush().await.unwrap();

    let e3 = quote_group("Bar 3", Some(&e1));
    let e4 = quote_group("Foo 4", Some(&e2));

    // Create
    em.persist(&e3).unwrap();
    em.persist(&e4).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let ids = [&e1, &e2, &e3, &e4].map(|e| get(e, "id"));

    // Retrieve
    let mut es = Vec::new();
    for id in &ids {
        es.push(em.find(QUOTE_GROUP, id.clone()).await.unwrap().unwrap());
    }

    for (e, id) in es.iter().zip(&ids) {
        assert_eq!(QUOTE_GROUP, e.class());
        assert_loose_eq(id, &get(e, "id"));
    }

    let names = ["Parent Bar 1", "Parent Foo 2", "Bar 3", "Foo 4"];
    for (e, name) in es.iter().zip(names) {
        assert_loose_eq(&s(name), &get(e, "name"));
    }

    let new_names = ["Parent Bar 11", "Parent Foo 22", "Bar 33", "Foo 44"];
    for (e, name) in es.iter().zip(new_names) {
        set(e, "name", s(name));
    }

    // Update
    for e in &es {
        em.persist(e).unwrap();
    }
    em.flush().await.unwrap();

    for (e, name) in es.iter().zip(new_names) {
        assert_loose_eq(&s(name), &get(e, "name"));
    }

    for (e, id) in es.iter().zip(&ids) {
        assert_eq!(QUOTE_GROUP, e.class());
        assert_loose_eq(id, &get(e, "id"));
    }

    for (e, name) in es.iter().zip(new_names) {
        assert_loose_eq(&s(name), &get(e, "name"));
    }

    // Delete
    for e in es.iter().rev() {
        em.remove(e).await.unwrap();
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    for e in &es {
        assert_eq!(QUOTE_GROUP, e.class());
    }

    // Retrieve
    for id in ids {
        assert!(em.find(QUOTE_GROUP, id).await.unwrap().is_none());
    }
}
