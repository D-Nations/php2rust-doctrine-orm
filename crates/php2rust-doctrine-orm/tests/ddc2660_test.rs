//! Ticket/DDC2660Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! `createSchema()` does not fail on `SQLite`, so `setUp()`'s
//! `catch (Exception)` has nothing to catch.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2660Test.php

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

use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ordered_map::OrderedMap;
use php2rust_doctrine_orm::query::ResultSetMappingBuilder;

use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["DDC2660Product", "DDC2660Customer", "DDC2660CustomerOrder"]).await;

    for i in 0..5 {
        let product = new("DDC2660Product");
        let customer = new("DDC2660Customer");
        // new DDC2660CustomerOrder($product, $customer, 'name' . $i)
        let order = new("DDC2660CustomerOrder");
        set_obj(&order, "product", &product);
        set_obj(&order, "customer", &customer);
        set(&order, "name", s(&format!("name{i}")));

        em.persist(&product).unwrap();
        em.persist(&customer).unwrap();
        em.flush().await.unwrap();

        em.persist(&order).unwrap();
        em.flush().await.unwrap();
    }

    em.clear().unwrap();
    em
}

async fn assert_orders(em: &Arc<EntityManager>, sql: &str) {
    let mut rsm = ResultSetMappingBuilder::new(em, None);
    rsm.add_root_entity_from_class_metadata(
        &class("DDC2660CustomerOrder"),
        "c",
        &OrderedMap::new(),
        None,
    )
    .unwrap();

    let query = em
        .create_native_query(sql, rsm.into_result_set_mapping())
        .unwrap();
    let result = list(&query.get_object_result().await.unwrap());

    assert_eq!(5, result.len());

    for order in &result {
        assert_ne!(&php_value::Value::Null, order);
        assert_instance_of("DDC2660CustomerOrder", order);
    }
}

#[tokio::test]
async fn testIssueWithExtraColumn() {
    let em = set_up().await;
    assert_orders(
        &em,
        "SELECT o.product_id, o.customer_id, o.name FROM ddc_2660_customer_order o",
    )
    .await;
}

#[tokio::test]
async fn testIssueWithoutExtraColumn() {
    let em = set_up().await;
    assert_orders(
        &em,
        "SELECT o.product_id, o.customer_id FROM ddc_2660_customer_order o",
    )
    .await;
}
