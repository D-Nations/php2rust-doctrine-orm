//! Ticket/DDC656Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC656Test.php

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

use php_value::{Key, PhpArray};

use orm::*;
use ticket::*;

fn keys(array: &PhpArray) -> Vec<Key> {
    array.keys().cloned().collect()
}

#[tokio::test]
async fn testRecomputeSingleEntityChangeSetPreservesFieldOrder() {
    let em = ticket::set_up(&["DDC656Entity"]).await;

    let entity = new("DDC656Entity");
    set(&entity, "name", s("test1"));
    set(&entity, "type", s("type1"));
    em.persist(&entity).unwrap();

    let class = em.get_class_metadata(&class("DDC656Entity")).unwrap();
    let uow = em.get_unit_of_work();
    uow.compute_change_set(&class, &entity).await.unwrap();
    let data1 = uow.get_entity_change_set(&entity);
    set(&entity, "type", s("type2"));
    uow.recompute_single_entity_change_set(&class, &entity)
        .unwrap();
    let data2 = uow.get_entity_change_set(&entity);

    assert_eq!(keys(&data1), keys(&data2));

    em.flush().await.unwrap();
    em.clear().unwrap();

    let persisted_entity = find(&em, "DDC656Entity", get(&entity, "specificationId"))
        .await
        .unwrap();
    assert_eq!(s("type2"), get(&persisted_entity, "type"));
    assert_eq!(s("test1"), get(&persisted_entity, "name"));
}
