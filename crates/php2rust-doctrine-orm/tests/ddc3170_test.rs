//! Ticket/DDC3170Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3170Test.php

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

use php2rust_doctrine_orm::abstract_query::HYDRATE_SIMPLEOBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&[
        "DDC3170AbstractEntityJoined",
        "DDC3170ProductJoined",
        "DDC3170AbstractEntitySingleTable",
        "DDC3170ProductSingleTable",
    ])
    .await;

    let product_joined = new("DDC3170ProductJoined");
    let product_single_table = new("DDC3170ProductSingleTable");
    em.persist(&product_joined).unwrap();
    em.persist(&product_single_table).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    for short in ["DDC3170ProductJoined", "DDC3170ProductSingleTable"] {
        let mut qb = em.create_query_builder();
        qb.select(vec![s("p")])
            .unwrap()
            .from(&class(short), "p", None)
            .unwrap();
        let result = list(
            &qb.get_query()
                .unwrap()
                .get_result(HYDRATE_SIMPLEOBJECT)
                .await
                .unwrap(),
        );

        assert_eq!(1, result.len());
        for value in &result {
            assert_eq!(class(short), obj(value).class(), "assertContainsOnly");
        }
    }
}
