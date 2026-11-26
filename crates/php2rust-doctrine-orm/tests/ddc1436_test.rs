//! Ticket/DDC1436Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1436Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIdentityMap() {
    let em = ticket::set_up(&["DDC1436Page"]).await;

    // fixtures
    let mut parent = Value::Null;
    for _ in 0..3 {
        let page = new("DDC1436Page");
        set(&page, "parent", parent);
        em.persist(&page).unwrap();
        parent = Value::Object(page);
    }

    em.flush().await.unwrap();
    em.clear().unwrap();

    let id = get(&obj(&parent), "id");

    // step 1
    let query = em.create_query(&format!(
        "SELECT p, parent FROM {NS}DDC1436Page p LEFT JOIN p.parent parent WHERE p.id = :id"
    ));
    query.set_parameter("id", id.clone(), None);
    let page = query.get_one_or_null_result(None).await.unwrap();

    assert_instance_of("DDC1436Page", &page);

    // step 2
    let page = find(&em, "DDC1436Page", id).await.unwrap();
    assert_instance_of("DDC1436Page", &Value::Object(page.clone()));
    let page_parent = get(&page, "parent");
    assert_instance_of("DDC1436Page", &page_parent);
    assert_instance_of("DDC1436Page", &get(&obj(&page_parent), "parent"));
}
