//! Ticket/DDC809Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC809Test.php

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
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC809Variant", "DDC809SpecificationValue"]).await;

    let conn = em.get_connection();
    let insert = async |table: &str, row: &[(&str, i64)]| {
        let row: Vec<(&str, Value)> = row.iter().map(|(k, v)| (*k, Value::Int(*v))).collect();
        let Value::Array(data) = array(&row) else {
            unreachable!()
        };
        conn.insert(table, &data, &ParameterTypes::new())
            .await
            .unwrap();
    };
    for id in [94589, 94593, 94606, 94607, 94609, 94711] {
        insert(
            "specification_value_test",
            &[("specification_value_id", id)],
        )
        .await;
    }
    insert("variant_test", &[("variant_id", 545_208)]).await;
    insert("variant_test", &[("variant_id", 545_209)]).await;
    for (variant, value) in [
        (545_208, 94606),
        (545_208, 94607),
        (545_208, 94609),
        (545_208, 94711),
        (545_209, 94589),
        (545_209, 94593),
        (545_209, 94606),
        (545_209, 94607),
    ] {
        insert(
            "var_spec_value_test",
            &[("variant_id", variant), ("specification_value_id", value)],
        )
        .await;
    }

    let mut qb = em.create_query_builder();
    qb.select(vec![s("Variant, SpecificationValue")])
        .unwrap()
        .from(&class("DDC809Variant"), "Variant", None)
        .unwrap()
        .left_join(
            "Variant.specificationValues",
            "SpecificationValue",
            None,
            None,
            None,
        )
        .unwrap();
    let result = list(
        &qb.get_query()
            .unwrap()
            .get_result(HYDRATE_OBJECT)
            .await
            .unwrap(),
    );

    assert_eq!(
        4,
        count_of(&obj(&result[0]), "specificationValues").await,
        "Works in test-setup."
    );
    assert_eq!(
        4,
        count_of(&obj(&result[1]), "specificationValues").await,
        "Only returns 2 in the case of the hydration bug."
    );
}
