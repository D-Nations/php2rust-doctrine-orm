//! Ticket/GH9807Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models; upstream's `ArrayResultFactory` result set is the rows that
//! `common/hydration.rs` hands to the object hydrator).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9807Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/hydration.rs"]
mod hydration;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::query::ResultSetMapping;
use php2rust_doctrine_orm::unit_of_work::Hints;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testHydrateJoinedCollectionWithFirstNullishRow() {
    let em = ticket::set_up(&["GH9807Main", "GH9807Join"]).await;
    let main = class("GH9807Main");
    let join = class("GH9807Join");

    let mut rsm = ResultSetMapping::new();
    rsm.add_entity_result(&main, "m", None);
    rsm.add_joined_entity_result(&join, "j", "m", "joins", false);
    rsm.add_field_result("m", "id_0", "id", None);
    rsm.add_field_result("j", "id_1", "id", None);
    rsm.add_field_result("j", "value_2", "value", None);

    let uow = em.get_unit_of_work();
    let mut data = PhpArray::new();
    data.insert("id", Value::Int(1));
    uow.create_entity(&main, data, &mut Hints::default())
        .await
        .unwrap();

    let result_set = hydration::rows(&[
        &[("id_0", Value::Int(1)), ("id_1", Value::Null), ("value_2", Value::Null)],
        &[("id_0", Value::Int(1)), ("id_1", Value::Int(1)), ("value_2", s("2"))],
        &[("id_0", Value::Int(1)), ("id_1", Value::Int(2)), ("value_2", s("2"))],
    ]);

    let result = list(
        &hydration::hydrate_all(&em, HYDRATE_OBJECT, result_set, &rsm)
            .await
            .unwrap(),
    );

    assert_instance_of("GH9807Main", &result[0]);
    assert_eq!(2, coll::count(&get(&obj(&result[0]), "joins")).await.unwrap());
}
