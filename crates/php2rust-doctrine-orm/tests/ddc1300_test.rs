//! Ticket/DDC1300Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1300Test.php

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

use php_value::PhpArray;
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC1300Foo", "DDC1300FooLocale"]).await;

    let foo = new("DDC1300Foo");
    set(
        &foo,
        "fooLocaleRefFoo",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&foo, "fooReference", s("foo"));

    em.persist(&foo).unwrap();
    em.flush().await.unwrap();

    let locale = new("DDC1300FooLocale");
    set_obj(&locale, "foo", &foo);
    set(&locale, "locale", s("en"));
    set(&locale, "title", s("blub"));

    em.persist(&locale).unwrap();
    em.flush().await.unwrap();

    let query = em.create_query(
        "SELECT f, fl FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1300Foo f JOIN f.fooLocaleRefFoo fl",
    );
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(1, result.len());
}
