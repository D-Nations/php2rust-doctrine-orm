//! Ticket/DDC2575Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2575Test.php

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

use php_value::{Object, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;

use orm::*;
use ticket::*;

/// `new $class($id)` of `DDC2575Root` or `DDC2575B` (`$sampleField = 0`).
fn with_id(short: &str, id: i64) -> Object {
    let object = new(short);
    set(&object, "id", Value::Int(id));
    set(&object, "sampleField", Value::Int(0));
    object
}

/// `new DDC2575A($rootRelation, $bRelation)`.
fn entity_a(root: &Object, b: &Object) -> Object {
    let a = new("DDC2575A");
    set_obj(&a, "rootRelation", root);
    set_obj(&a, "bRelation", b);
    a
}

#[tokio::test]
#[allow(clippy::similar_names, reason = "upstream's names")]
async fn testHydrationIssue() {
    let em = ticket::set_up(&["DDC2575Root", "DDC2575A", "DDC2575B"]).await;

    let entity_root1 = with_id("DDC2575Root", 1);
    let entity_b1 = with_id("DDC2575B", 2);
    let entity_a1 = entity_a(&entity_root1, &entity_b1);

    em.persist(&entity_root1).unwrap();
    em.persist(&entity_a1).unwrap();
    em.persist(&entity_b1).unwrap();

    let entity_root2 = with_id("DDC2575Root", 3);
    let entity_b2 = with_id("DDC2575B", 4);
    let entity_a2 = entity_a(&entity_root2, &entity_b2);

    em.persist(&entity_root2).unwrap();
    em.persist(&entity_a2).unwrap();
    em.persist(&entity_b2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let repository = em.get_repository(&class("DDC2575Root")).unwrap();
    let mut qb = repository.create_query_builder("r", None).unwrap();
    qb.select(vec![s("r, a, b")])
        .unwrap()
        .left_join("r.aRelation", "a", None, None, None)
        .unwrap()
        .left_join("a.bRelation", "b", None, None, None)
        .unwrap();

    let query = qb.get_query().unwrap();
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(2, result.len());

    for (row, id, b_id) in [(&result[0], 1, 2), (&result[1], 3, 4)] {
        let row = obj(row);
        let a_relation = get(&row, "aRelation");
        assert!(!matches!(a_relation, Value::Null));
        assert_loose_eq(&Value::Int(id), &get(&row, "id"));
        let a_relation = obj(&a_relation);
        assert!(!matches!(get(&a_relation, "rootRelation"), Value::Null));
        assert!(same_value(&get(&a_relation, "rootRelation"), &row));
        assert!(!matches!(get(&a_relation, "bRelation"), Value::Null));
        assert_loose_eq(
            &Value::Int(b_id),
            &get(&obj(&get(&a_relation, "bRelation")), "id"),
        );
    }
}
