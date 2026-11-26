//! Ticket/GH8127Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). The data provider's rows run in one test, each on a
//! new database.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8127Test.php

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

use orm::*;
use ticket::*;

/// `queryClasses()`.
const QUERY_CLASSES: [(&str, &str); 2] = [
    ("query via root entity", "GH8127Root"),
    ("query via leaf entity", "GH8127Leaf"),
];

#[tokio::test]
async fn testLoadFieldsFromAllClassesInHierarchy() {
    for (_, query_class) in QUERY_CLASSES {
        let em = ticket::set_up(&["GH8127Root", "GH8127Middle", "GH8127Leaf"]).await;
        let entity = new("GH8127Leaf");
        set(&entity, "root", s("root"));
        set(&entity, "middle", s("middle"));
        set(&entity, "leaf", s("leaf"));

        em.persist(&entity).unwrap();
        em.flush().await.unwrap();
        em.clear().unwrap();

        let loaded_entity = find(&em, query_class, get(&entity, "id")).await.unwrap();

        assert!(s("root").identical(&get(&loaded_entity, "root")));
        assert!(s("middle").identical(&get(&loaded_entity, "middle")));
        assert!(s("leaf").identical(&get(&loaded_entity, "leaf")));
    }
}
