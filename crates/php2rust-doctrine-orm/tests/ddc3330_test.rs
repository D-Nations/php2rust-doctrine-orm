//! Ticket/DDC3330Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3330Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssueCollectionOrderWithPaginator() {
    let em = ticket::set_up(&["DDC3330Building", "DDC3330Hall"]).await;

    create_building_and_halls(&em).await;
    create_building_and_halls(&em).await;
    create_building_and_halls(&em).await;

    em.clear().unwrap();

    let query = em.create_query(&format!(
        "SELECT b, h FROM {NS}DDC3330Building b LEFT JOIN b.halls h ORDER BY b.id ASC, h.name DESC"
    ));

    let page = OffsetPaginator::new(true, None)
        .paginate_window((&query).into(), Window::new(0, 3).unwrap())
        .await
        .unwrap();

    assert_eq!(3, page.count(), "Count is not correct for pagination");
}

/// Create a building and 10 halls
async fn create_building_and_halls(em: &Arc<EntityManager>) {
    let building = new("DDC3330Building");

    for i in 0..10 {
        let hall = new("DDC3330Hall");
        set(&hall, "name", s(&format!("HALL-{i}")));
        // DDC3330Building::addHall()
        append(&building, "halls", Value::Object(hall.clone()));
        set_obj(&hall, "building", &building);
    }

    em.persist(&building).unwrap();
    em.flush().await.unwrap();
}
