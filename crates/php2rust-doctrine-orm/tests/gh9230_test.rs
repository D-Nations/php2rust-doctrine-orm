//! Ticket/GH9230Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models). The data providers' rows run in one test, each on a
//! new database, so upstream's `tearDown()` that deletes the rows has no
//! counterpart.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9230Test.php

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

use php_value::{PhpArray, Value};

use orm::*;
use ticket::*;

fn failing_values_before_fix() -> Vec<(&'static str, &'static str, Value, Value)> {
    vec![
        ("string=\"\"", "name", s(""), s("test name")),
        ("string=\"0\"", "name", s("0"), s("test name")),
        ("string=null", "name", Value::Null, s("test name")),
        ("int=0", "counter", Value::Int(0), Value::Int(1)),
        ("int=null", "counter", Value::Null, Value::Int(1)),
        ("bool=false", "enabled", Value::Bool(false), Value::Bool(true)),
        ("bool=null", "enabled", Value::Null, Value::Bool(true)),
        ("float=0.0", "price", Value::Float(0.0), Value::Float(1.1)),
        ("float=-0.0", "price", Value::Float(-0.0), Value::Float(1.1)),
        ("float=null", "price", Value::Null, Value::Float(1.1)),
        ("json=[]", "extra", Value::Array(PhpArray::new()), Value::Int(1)),
        ("json=0", "extra", Value::Int(0), Value::Int(1)),
        ("json=0.0", "extra", Value::Float(0.0), Value::Int(1)),
        ("json=false", "extra", Value::Bool(false), Value::Int(1)),
        ("json=\"\"", "extra", s(""), Value::Int(1)),
        ("json=null", "enabled", Value::Null, Value::Int(1)),
    ]
}

fn succeeding_values_before_fix() -> Vec<(&'static str, &'static str, Value, Value)> {
    vec![
        ("string=\"test\"", "name", s("test"), s("test2")),
        ("int=1", "counter", Value::Int(1), Value::Int(1)),
        ("bool=true", "enabled", Value::Bool(true), Value::Int(1)),
        (
            "json=[null]",
            "extra",
            Value::Array(PhpArray::list(vec![Value::Null])),
            Value::Int(1),
        ),
        ("json=1", "extra", Value::Int(1), Value::Int(1)),
        ("json=1.1", "extra", Value::Float(1.1), Value::Int(1)),
        ("json=true", "extra", Value::Bool(true), Value::Int(1)),
        ("json=\"test\"", "extra", s("test"), s("test")),
        ("json=\"1\"", "extra", s("1"), Value::Int(1)),
    ]
}

#[tokio::test]
async fn testIssue() {
    for (name, property, falsy_value, truthy_value) in failing_values_before_fix()
        .into_iter()
        .chain(succeeding_values_before_fix())
    {
        let em = ticket::set_up(&["GH9230Entity"]).await;
        let counter1 = new("GH9230Entity");
        set(&counter1, property, falsy_value.clone());
        let counter2 = new("GH9230Entity");
        set(&counter2, property, truthy_value.clone());

        em.persist(&counter1).unwrap();
        em.persist(&counter2).unwrap();
        em.flush().await.unwrap();
        em.clear().unwrap();

        let persisted_counter1 = find(&em, "GH9230Entity", get(&counter1, "id"))
            .await
            .unwrap();
        let persisted_counter2 = find(&em, "GH9230Entity", get(&counter2, "id"))
            .await
            .unwrap();

        // Assert entities were persisted
        assert_instance_of("GH9230Entity", &Value::Object(persisted_counter1.clone()));
        assert_instance_of("GH9230Entity", &Value::Object(persisted_counter2.clone()));
        assert_loose_eq(&falsy_value, &get(&persisted_counter1, property));
        assert_loose_eq(&truthy_value, &get(&persisted_counter2, property));

        em.clear().unwrap();

        let counter_repository = em.get_repository(&class("GH9230Entity")).unwrap();
        let mut qb = counter_repository.create_query_builder("counter", None).unwrap();
        qb.select(vec![s(&format!("counter.{property}"))]).unwrap();
        let query = qb.get_query().unwrap();
        let values = query.get_single_column_result().await.unwrap();

        // Assert that there are 2 values returned.
        assert_eq!(2, list(&values).len(), "{name}");
    }
}
