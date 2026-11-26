//! Ticket/DDC237Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC237Test.php

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

#[tokio::test]
async fn testUninitializedProxyIsInitializedOnFetchJoin() {
    let em = ticket::set_up(&["DDC237EntityX", "DDC237EntityY", "DDC237EntityZ"]).await;

    let x = new("DDC237EntityX");
    let y = new("DDC237EntityY");
    let z = new("DDC237EntityZ");

    set(&x, "data", s("X"));
    set(&y, "data", s("Y"));
    set(&z, "data", s("Z"));

    set_obj(&x, "y", &y);
    set_obj(&z, "y", &y);

    em.persist(&x).unwrap();
    em.persist(&y).unwrap();
    em.persist(&z).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let x2 = find(&em, "DDC237EntityX", get(&x, "id")).await.unwrap(); // proxy injected for Y
    assert!(is_uninitialized(&em, &get(&x2, "y")));

    let query = em.create_query(&format!(
        "select z,y from {} z join z.y y where z.id = ?1",
        class("DDC237EntityZ")
    ));
    query.set_parameter(1, get(&z, "id"), None);
    let z2 = obj(&query.get_single_result(None).await.unwrap());

    let z2_y = get(&z2, "y");
    assert!(!is_uninitialized(&em, &z2_y));
    assert_loose_eq(&s("Y"), &get(&obj(&z2_y), "data"));
    assert_loose_eq(&get(&y, "id"), &get(&obj(&z2_y), "id"));

    assert!(!same(&x, &x2));
    assert!(!same(&z, &z2));
    assert!(same_value(&get(&x2, "y"), &obj(&z2_y)));
}
