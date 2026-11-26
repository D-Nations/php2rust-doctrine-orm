//! Ticket/DDC522Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `assertEquals($expected, $found)` compares the mapped properties.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC522Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    ticket::set_up(&["DDC522Customer", "DDC522Cart", "DDC522ForeignKeyTest"]).await
}

#[tokio::test]
async fn testJoinColumnWithSameNameAsAssociationField() {
    let em = set_up().await;

    let cust = new("DDC522Customer");
    set(&cust, "name", s("name"));
    let cart = new("DDC522Cart");
    set(&cart, "total", Value::Int(0));
    set_obj(&cust, "cart", &cart);
    set_obj(&cart, "customer", &cust);

    em.persist(&cust).unwrap();
    em.persist(&cart).unwrap();
    em.flush().await.unwrap();

    em.clear().unwrap();

    let r = list(
        &em.create_query(&format!(
            "select ca,c from {} ca join ca.customer c",
            class("DDC522Cart")
        ))
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap(),
    );

    assert_instance_of("DDC522Cart", &r[0]);
    let r0 = obj(&r[0]);
    assert_instance_of("DDC522Customer", &get(&r0, "customer"));
    assert!(!is_uninitialized(&em, &get(&r0, "customer")));
    assert_eq!(s("name"), get(&obj(&get(&r0, "customer")), "name"));

    let fkt = new("DDC522ForeignKeyTest");
    set(&fkt, "cartId", get(&r0, "id")); // ignored for persistence
    set_obj(&fkt, "cart", &r0); // must be set properly

    em.persist(&fkt).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let fkt2 = find(&em, "DDC522ForeignKeyTest", get(&fkt, "id"))
        .await
        .unwrap();
    assert_loose_eq(&get(&obj(&get(&fkt, "cart")), "id"), &get(&fkt2, "cartId"));
    assert!(is_uninitialized(&em, &get(&fkt2, "cart")));
}

#[tokio::test]
async fn testJoinColumnWithNullSameNameAssociationField() {
    let em = set_up().await;

    let fk_cust = new("DDC522ForeignKeyTest");
    set(&fk_cust, "cart", Value::Null);

    em.persist(&fk_cust).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let found = find(&em, "DDC522ForeignKeyTest", get(&fk_cust, "id"))
        .await
        .unwrap();
    for property in ["id", "cartId", "cart"] {
        assert_loose_eq(&get(&fk_cust, property), &get(&found, property));
    }
}
