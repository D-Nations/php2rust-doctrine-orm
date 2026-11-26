//! Ticket/DDC258Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC258Test.php

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
async fn testIssue() {
    let em = ticket::set_up(&[
        "DDC258Super",
        "DDC258Class1",
        "DDC258Class2",
        "DDC258Class3",
    ])
    .await;

    let c1 = new("DDC258Class1");
    set(&c1, "title", s("Foo"));
    set(&c1, "description", s("Foo"));

    let c2 = new("DDC258Class2");
    set(&c2, "title", s("Bar"));
    set(&c2, "description", s("Bar"));
    set(&c2, "text", s("Bar"));

    let c3 = new("DDC258Class3");
    set(&c3, "apples", s("Baz"));
    set(&c3, "bananas", s("Baz"));

    em.persist(&c1).unwrap();
    em.persist(&c2).unwrap();
    em.persist(&c3).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let e2 = find(&em, "DDC258Super", get(&c2, "id")).await.unwrap();

    assert_instance_of("DDC258Class2", &Value::Object(e2.clone()));
    assert_eq!(s("Bar"), get(&e2, "title"));
    assert_eq!(s("Bar"), get(&e2, "description"));
    assert_eq!(s("Bar"), get(&e2, "text"));

    let all = find_all(&em, "DDC258Super").await;

    for obj in all {
        if obj.instance_of(&class("DDC258Class1")) {
            assert_eq!(s("Foo"), get(&obj, "title"));
            assert_eq!(s("Foo"), get(&obj, "description"));
        } else if obj.instance_of(&class("DDC258Class2")) {
            assert!(same(&e2, &obj));
            assert_eq!(s("Bar"), get(&obj, "title"));
            assert_eq!(s("Bar"), get(&obj, "description"));
            assert_eq!(s("Bar"), get(&obj, "text"));
        } else if obj.instance_of(&class("DDC258Class3")) {
            assert_eq!(s("Baz"), get(&obj, "apples"));
            assert_eq!(s("Baz"), get(&obj, "bananas"));
        } else {
            panic!("Instance of DDC258Class1, DDC258Class2 or DDC258Class3 expected.");
        }
    }
}
