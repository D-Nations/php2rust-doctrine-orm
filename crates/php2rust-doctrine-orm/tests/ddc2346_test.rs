//! Ticket/DDC2346Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2346Test.php

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
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/ticket.rs"]
mod ticket;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let classes = [
        class("DDC2346Foo"),
        class("DDC2346Bar"),
        class("DDC2346Baz"),
    ];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let foo1 = new("DDC2346Foo");
    set(&foo1, "bars", ArrayCollectionObject::value(PhpArray::new()));
    let foo2 = new("DDC2346Foo");
    set(&foo2, "bars", ArrayCollectionObject::value(PhpArray::new()));

    let baz1 = new("DDC2346Baz");
    let baz2 = new("DDC2346Baz");

    set_obj(&baz1, "foo", &foo1);
    set_obj(&baz2, "foo", &foo2);

    coll::add(&get(&foo1, "bars"), Value::Object(baz1.clone())).unwrap();
    coll::add(&get(&foo1, "bars"), Value::Object(baz2.clone())).unwrap();

    em.persist(&foo1).unwrap();
    em.persist(&foo2).unwrap();
    em.persist(&baz1).unwrap();
    em.persist(&baz2).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    log.reset_and_enable();

    let fetched_bazs = find_all(&em, "DDC2346Baz").await;

    assert_eq!(2, fetched_bazs.len());
    assert_eq!(
        2,
        log.queries().len(),
        "The total number of executed queries is 2, and not n+1"
    );
}
