//! Ticket/DDC1080Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1080Test.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::*;
use ticket::*;

/// `new $class()` with its `fooBars` collection.
fn with_foo_bars(short: &str, title_field: &str, title: &str) -> Object {
    let object = new(short);
    set(
        &object,
        "fooBars",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&object, title_field, s(title));
    object
}

/// `new DDC1080FooBar()`, `setFoo()`, `setBar()`, `setOrderNr()`.
fn foo_bar(foo: &Object, bar: &Object, order_nr: i64) -> Object {
    let foobar = new("DDC1080FooBar");
    set_obj(&foobar, "foo", foo);
    set_obj(&foobar, "bar", bar);
    set(&foobar, "orderNr", Value::Int(order_nr));
    foobar
}

#[tokio::test]
async fn testHydration() {
    let em = ticket::set_up(&["DDC1080Foo", "DDC1080Bar", "DDC1080FooBar"]).await;

    let foo1 = with_foo_bars("DDC1080Foo", "fooTitle", "foo title 1");
    let foo2 = with_foo_bars("DDC1080Foo", "fooTitle", "foo title 2");
    let bar1 = with_foo_bars("DDC1080Bar", "barTitle", "bar title 1");
    let bar2 = with_foo_bars("DDC1080Bar", "barTitle", "bar title 2");
    let bar3 = with_foo_bars("DDC1080Bar", "barTitle", "bar title 3");

    let foobar1 = foo_bar(&foo1, &bar1, 0);
    let foobar2 = foo_bar(&foo1, &bar2, 0);
    let foobar3 = foo_bar(&foo1, &bar3, 0);

    em.persist(&foo1).unwrap();
    em.persist(&foo2).unwrap();
    em.persist(&bar1).unwrap();
    em.persist(&bar2).unwrap();
    em.persist(&bar3).unwrap();
    em.flush().await.unwrap();

    em.persist(&foobar1).unwrap();
    em.persist(&foobar2).unwrap();
    em.persist(&foobar3).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let foo = find(&em, "DDC1080Foo", get(&foo1, "fooID")).await.unwrap();
    assert_eq!(
        3,
        count_of(&foo, "fooBars").await,
        "Should return three foobars."
    );
}
