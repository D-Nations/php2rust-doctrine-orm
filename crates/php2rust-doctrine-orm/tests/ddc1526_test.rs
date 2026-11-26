//! Ticket/DDC1526Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1526Test.php

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

use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC1526Menu"]).await;

    let mut parents = Vec::new();
    for i in 0..9 {
        let entity = new("DDC1526Menu");
        if let Some(parent) = parents.get(i % 3) {
            set_obj(&entity, "parent", parent);
        }
        em.persist(&entity).unwrap();
        parents.push(entity);
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!(
        "SELECT m, c
            FROM {NS}DDC1526Menu m
            LEFT JOIN m.children c"
    );
    let menus = list(
        &em.create_query(&dql)
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    for menu in &menus {
        assert!(
            PersistentCollection::from_value(&get(&obj(menu), "children"))
                .unwrap()
                .is_initialized()
        );
    }
}
