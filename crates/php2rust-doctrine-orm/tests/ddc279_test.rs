//! Ticket/DDC279Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC279Test.php

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

use orm::*;
use ticket::*;

#[tokio::test]
async fn testDDC279() {
    let em = ticket::set_up(&[
        "DDC279EntityXAbstract",
        "DDC279EntityX",
        "DDC279EntityY",
        "DDC279EntityZ",
    ])
    .await;

    let x = new("DDC279EntityX");
    let y = new("DDC279EntityY");
    let z = new("DDC279EntityZ");

    set(&x, "data", s("X"));
    set(&y, "data", s("Y"));
    set(&z, "data", s("Z"));

    set_obj(&x, "y", &y);
    set_obj(&y, "z", &z);

    em.persist(&x).unwrap();
    em.persist(&y).unwrap();
    em.persist(&z).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "SELECT x, y, z FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC279EntityX x INNER JOIN x.y y INNER JOIN y.z z WHERE x.id = ?1",
    );
    query.set_parameter(1, get(&x, "id"), None);

    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    let expected1 = s("Y");
    let expected2 = s("Z");

    assert_eq!(1, result.len());

    let result_y = get_object(&obj(&result[0]), "y").unwrap();
    assert_loose_eq(&expected1, &get(&result_y, "data"));
    assert_loose_eq(
        &expected2,
        &get(&get_object(&result_y, "z").unwrap(), "data"),
    );
}
