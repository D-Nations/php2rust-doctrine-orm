//! Ticket/DDC512Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC512Test.php

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

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC512Customer", "DDC512OfferItem", "DDC512Item"]).await;

    let customer1 = new("DDC512Customer");
    let item = new("DDC512OfferItem");
    set_obj(&customer1, "item", &item);
    em.persist(&customer1).unwrap();

    let customer2 = new("DDC512Customer");
    em.persist(&customer2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let q = em.create_query(&format!(
        "select u,i from {NS}DDC512Customer u left join u.item i"
    ));
    let result = list(&q.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(2, result.len());
    assert_instance_of("DDC512Customer", &result[0]);
    assert_instance_of("DDC512Customer", &result[1]);
    let (r0, r1) = (obj(&result[0]), obj(&result[1]));
    if get(&r0, "id") == get(&customer1, "id") {
        assert_instance_of("DDC512OfferItem", &get(&r0, "item"));
        assert_loose_eq(&get(&item, "id"), &get(&obj(&get(&r0, "item")), "id"));
        assert!(matches!(get(&r1, "item"), Value::Null));
    } else {
        assert_instance_of("DDC512OfferItem", &get(&r1, "item"));
        assert!(matches!(get(&r0, "item"), Value::Null));
    }
}
