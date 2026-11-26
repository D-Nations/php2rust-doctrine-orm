//! Ticket/GH10625Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! A data provider's rows run in one test.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH10625Test.php

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

use ticket::*;

/// `queryClasses()`.
const QUERY_CLASSES: [(&str, &str); 3] = [
    ("query via root entity", "GH10625Root"),
    ("query via intermediate entity", "GH10625Middle"),
    ("query via leaf entity", "GH10625Leaf"),
];

#[tokio::test]
async fn testLoadFieldsFromAllClassesInHierarchy() {
    for (name, query_class) in QUERY_CLASSES {
        let em = ticket::set_up(&["GH10625Root", "GH10625Middle", "GH10625Leaf"]).await;

        let entity = new("GH10625Leaf");
        em.persist(&entity).unwrap();
        em.flush().await.unwrap();
        em.clear().unwrap();

        let loaded_entity = find(&em, query_class, orm::get(&entity, "id")).await;
        let loaded_entity = loaded_entity.unwrap_or_else(|| panic!("{name}"));
        assert_instance_of("GH10625Leaf", &php_value::Value::Object(loaded_entity));
    }
}
