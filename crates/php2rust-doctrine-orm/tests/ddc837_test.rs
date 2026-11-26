//! Ticket/DDC837Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! `initialize_object()` where PHP calls `getSysname()` on a proxy.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC837Test.php

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

/// `new DDC837Aggregate($sysname)`.
fn aggregate(sysname: &str) -> php_value::Object {
    let aggregate = new("DDC837Aggregate");
    set(&aggregate, "sysname", s(sysname));
    aggregate
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&[
        "DDC837Super",
        "DDC837Class1",
        "DDC837Class2",
        "DDC837Class3",
        "DDC837Aggregate",
    ])
    .await;

    let c1 = new("DDC837Class1");
    set(&c1, "title", s("Foo"));
    set(&c1, "description", s("Foo"));
    let aggregate1 = aggregate("test1");
    set_obj(&c1, "aggregate", &aggregate1);

    let c2 = new("DDC837Class2");
    set(&c2, "title", s("Bar"));
    set(&c2, "description", s("Bar"));
    set(&c2, "text", s("Bar"));
    let aggregate2 = aggregate("test2");
    set_obj(&c2, "aggregate", &aggregate2);

    let c3 = new("DDC837Class3");
    set(&c3, "apples", s("Baz"));
    set(&c3, "bananas", s("Baz"));

    em.persist(&c1).unwrap();
    em.persist(&aggregate1).unwrap();
    em.persist(&c2).unwrap();
    em.persist(&aggregate2).unwrap();
    em.persist(&c3).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Test Class1
    let e1 = find(&em, "DDC837Super", get(&c1, "id")).await.unwrap();

    assert_instance_of("DDC837Class1", &Value::Object(e1.clone()));
    assert_eq!(s("Foo"), get(&e1, "title"));
    assert_eq!(s("Foo"), get(&e1, "description"));
    assert_instance_of("DDC837Aggregate", &get(&e1, "aggregate"));
    let aggregate = obj(&get(&e1, "aggregate"));
    em.initialize_object(&aggregate).await.unwrap();
    assert_eq!(s("test1"), get(&aggregate, "sysname"));

    // Test Class 2
    let e2 = find(&em, "DDC837Super", get(&c2, "id")).await.unwrap();

    assert_instance_of("DDC837Class2", &Value::Object(e2.clone()));
    assert_eq!(s("Bar"), get(&e2, "title"));
    assert_eq!(s("Bar"), get(&e2, "description"));
    assert_eq!(s("Bar"), get(&e2, "text"));
    assert_instance_of("DDC837Aggregate", &get(&e2, "aggregate"));
    let aggregate = obj(&get(&e2, "aggregate"));
    em.initialize_object(&aggregate).await.unwrap();
    assert_eq!(s("test2"), get(&aggregate, "sysname"));

    let all = find_all(&em, "DDC837Super").await;

    for obj in all {
        if obj.instance_of(&class("DDC837Class1")) {
            assert_eq!(s("Foo"), get(&obj, "title"));
            assert_eq!(s("Foo"), get(&obj, "description"));
        } else if obj.instance_of(&class("DDC837Class2")) {
            assert!(same(&e2, &obj));
            assert_eq!(s("Bar"), get(&obj, "title"));
            assert_eq!(s("Bar"), get(&obj, "description"));
            assert_eq!(s("Bar"), get(&obj, "text"));
        } else if obj.instance_of(&class("DDC837Class3")) {
            assert_eq!(s("Baz"), get(&obj, "apples"));
            assert_eq!(s("Baz"), get(&obj, "bananas"));
        } else {
            panic!("Instance of DDC837Class1, DDC837Class2 or DDC837Class3 expected.");
        }
    }
}
