//! Ticket/DDC1430Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `assertSQLEquals()` compares lower-cased strings. The
//! `Exception` catch of `setUp()` has nothing to catch.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1430Test.php

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

use php_value::datetime::{DateTime, DateTimeImmutable, TimeZone};
use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::exec::SqlStatements;

use orm::*;
use ticket::*;

/// `new DDC1430Order($status)`.
fn order(status: &str) -> Object {
    let order = new("DDC1430Order");
    set(&order, "status", s(status));
    set(
        &order,
        "date",
        DateTime::object(DateTimeImmutable::now(TimeZone::Utc)),
    );
    set(
        &order,
        "products",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    order
}

/// `$order->addProduct(new DDC1430OrderProduct($value))`.
fn add_product(order: &Object, value: f64) {
    let product = new("DDC1430OrderProduct");
    set(&product, "value", Value::Float(value));
    set_obj(&product, "order", order);
    coll::add(&get(order, "products"), Value::Object(product)).unwrap();
}

async fn set_up() -> Arc<EntityManager> {
    let em = ticket::set_up(&["DDC1430Order", "DDC1430OrderProduct"]).await;
    load_fixtures(&em).await;
    em
}

async fn load_fixtures(em: &EntityManager) {
    let o1 = order("NEW");
    let o2 = order("OK");

    add_product(&o1, 1.1);
    add_product(&o1, 1.2);

    add_product(&o2, 2.1);
    add_product(&o2, 2.2);
    add_product(&o2, 2.3);

    em.persist(&o1).unwrap();
    em.persist(&o2).unwrap();

    em.flush().await.unwrap();
}

fn assert_sql_equals(expected: &str, actual: &str) {
    assert_eq!(expected.to_lowercase(), actual.to_lowercase());
}

fn sql(query: &php2rust_doctrine_orm::query::Query) -> String {
    let SqlStatements::One(sql) = query.get_sql().unwrap() else {
        panic!("one statement");
    };
    sql
}

fn has_key(value: &Value, key: &str) -> bool {
    matches!(value, Value::Array(array) if array.get(&key.into()).is_some())
}

#[tokio::test]
async fn testOrderByFields() {
    let em = set_up().await;
    let repository = em.get_repository(&class("DDC1430Order")).unwrap();
    let mut builder = repository.create_query_builder("o", None).unwrap();
    builder
        .select(vec![s("o.id, o.date, COUNT(p.id) AS p_count")])
        .unwrap()
        .left_join("o.products", "p", None, None, None)
        .unwrap()
        .group_by(&["o.id, o.date"])
        .unwrap()
        .order_by("o.id", None)
        .unwrap();
    let query = builder.get_query().unwrap();

    assert_sql_equals(
        "SELECT o.id, o.date, COUNT(p.id) AS p_count FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1430Order o LEFT JOIN o.products p GROUP BY o.id, o.date ORDER BY o.id ASC",
        &query.get_dql().unwrap(),
    );
    assert_sql_equals(
        "SELECT d0_.order_id AS order_id_0, d0_.created_at AS created_at_1, COUNT(d1_.id) AS sclr_2 FROM DDC1430Order d0_ LEFT JOIN DDC1430OrderProduct d1_ ON d0_.order_id = d1_.order_id GROUP BY d0_.order_id, d0_.created_at ORDER BY d0_.order_id ASC",
        &sql(&query),
    );

    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(2, result.len());

    assert!(has_key(&result[0], "id"));
    assert!(has_key(&result[1], "id"));

    assert!(has_key(&result[0], "p_count"));
    assert!(has_key(&result[1], "p_count"));

    assert_loose_eq(&Value::Int(1), &at(&result[0], "id"));
    assert_loose_eq(&Value::Int(2), &at(&result[1], "id"));

    assert_loose_eq(&Value::Int(2), &at(&result[0], "p_count"));
    assert_loose_eq(&Value::Int(3), &at(&result[1], "p_count"));
}

/// The assertions of `testOrderByAllObjectFields()` and `testTicket()`
/// on the result.
async fn assert_order_result(query: &php2rust_doctrine_orm::query::Query) {
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(2, result.len());

    assert_instance_of("DDC1430Order", &at(&result[0], 0));
    assert_instance_of("DDC1430Order", &at(&result[1], 0));

    assert_loose_eq(&get(&obj(&at(&result[0], 0)), "id"), &Value::Int(1));
    assert_loose_eq(&get(&obj(&at(&result[1], 0)), "id"), &Value::Int(2));

    assert_loose_eq(&at(&result[0], "p_count"), &Value::Int(2));
    assert_loose_eq(&at(&result[1], "p_count"), &Value::Int(3));
}

#[tokio::test]
async fn testOrderByAllObjectFields() {
    let em = set_up().await;
    let repository = em.get_repository(&class("DDC1430Order")).unwrap();
    let mut builder = repository.create_query_builder("o", None).unwrap();
    builder
        .select(vec![s("o, COUNT(p.id) AS p_count")])
        .unwrap()
        .left_join("o.products", "p", None, None, None)
        .unwrap()
        .group_by(&["o.id, o.date, o.status"])
        .unwrap()
        .order_by("o.id", None)
        .unwrap();
    let query = builder.get_query().unwrap();

    assert_sql_equals(
        "SELECT o, COUNT(p.id) AS p_count FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1430Order o LEFT JOIN o.products p GROUP BY o.id, o.date, o.status ORDER BY o.id ASC",
        &query.get_dql().unwrap(),
    );
    assert_sql_equals(
        "SELECT d0_.order_id AS order_id_0, d0_.created_at AS created_at_1, d0_.order_status AS order_status_2, COUNT(d1_.id) AS sclr_3 FROM DDC1430Order d0_ LEFT JOIN DDC1430OrderProduct d1_ ON d0_.order_id = d1_.order_id GROUP BY d0_.order_id, d0_.created_at, d0_.order_status ORDER BY d0_.order_id ASC",
        &sql(&query),
    );

    assert_order_result(&query).await;
}

#[tokio::test]
async fn testTicket() {
    let em = set_up().await;
    let repository = em.get_repository(&class("DDC1430Order")).unwrap();
    let mut builder = repository.create_query_builder("o", None).unwrap();
    builder
        .select(vec![s("o, COUNT(p.id) AS p_count")])
        .unwrap()
        .left_join("o.products", "p", None, None, None)
        .unwrap()
        .group_by(&["o"])
        .unwrap()
        .order_by("o.id", None)
        .unwrap();
    let query = builder.get_query().unwrap();

    assert_sql_equals(
        "SELECT o, COUNT(p.id) AS p_count FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1430Order o LEFT JOIN o.products p GROUP BY o ORDER BY o.id ASC",
        &query.get_dql().unwrap(),
    );
    assert_sql_equals(
        "SELECT d0_.order_id AS order_id_0, d0_.created_at AS created_at_1, d0_.order_status AS order_status_2, COUNT(d1_.id) AS sclr_3 FROM DDC1430Order d0_ LEFT JOIN DDC1430OrderProduct d1_ ON d0_.order_id = d1_.order_id GROUP BY d0_.order_id, d0_.created_at, d0_.order_status ORDER BY d0_.order_id ASC",
        &sql(&query),
    );

    assert_order_result(&query).await;
}
