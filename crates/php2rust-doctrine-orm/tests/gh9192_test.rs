//! Ticket/GH9192Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH9192Test.php

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
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["GH9192A", "GH9192B", "GH9192C"]).await;

    let a = new("GH9192A");
    set(&a, "bs", ArrayCollectionObject::value(PhpArray::new()));

    let b = new("GH9192B");
    set(&b, "cs", ArrayCollectionObject::value(PhpArray::new()));
    set_obj(&b, "a", &a);
    coll::add(&get(&a, "bs"), Value::Object(b.clone())).unwrap();

    let c = new("GH9192C");
    set_obj(&c, "b", &b);
    coll::add(&get(&b, "cs"), Value::Object(c.clone())).unwrap();

    set_obj(&a, "c", &c);

    em.persist(&a).unwrap();
    em.persist(&b).unwrap();
    em.persist(&c).unwrap();
    em.flush().await.unwrap();

    em.remove(&a).await.unwrap();
    em.flush().await.unwrap();
}
