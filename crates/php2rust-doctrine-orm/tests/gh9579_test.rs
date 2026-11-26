//! Ticket/GH9579Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9579Test.php

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

use php_value::PhpArray;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["GH9579Container", "GH9579Item", "GH9579Part"]).await;

    let container = new("GH9579Container");
    set(&container, "items", ArrayCollectionObject::value(PhpArray::new()));

    let item = new("GH9579Item");
    set(&item, "parts", ArrayCollectionObject::value(PhpArray::new()));
    set_obj(&item, "container", &container);
    set_obj(&container, "currentItem", &item);

    let part = new("GH9579Part");
    set_obj(&part, "item", &item);

    em.persist(&item).unwrap();
    em.persist(&container).unwrap();
    em.persist(&part).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    em
}

#[tokio::test]
async fn testIssue() {
    let em = set_up().await;
    let dql = "SELECT container, currentItem, currentItemPart, item, itemPart
FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH9579Container container
LEFT JOIN container.currentItem currentItem
LEFT JOIN currentItem.parts currentItemPart
LEFT JOIN container.items item 
LEFT JOIN item.parts itemPart";
    let containers = list(&em.create_query(dql).execute(None, None).await.unwrap());

    assert_eq!(1, containers.len());
    let items = get(&obj(&containers[0]), "items");
    assert_eq!(1, coll::count(&items).await.unwrap());
    let item = coll::get(&items, 0).await.unwrap();
    assert_eq!(1, coll::count(&get(&obj(&item), "parts")).await.unwrap());
}
