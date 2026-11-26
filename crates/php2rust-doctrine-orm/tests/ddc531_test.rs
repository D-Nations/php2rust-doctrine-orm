//! Ticket/DDC531Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC531Test.php

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

/// `new DDC531Item()`.
fn item() -> php_value::Object {
    let item = new("DDC531Item");
    set(
        &item,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    item
}

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC531Item", "DDC531SubItem"]).await;

    let item1 = item();
    let item2 = item();
    set_obj(&item2, "parent", &item1);
    coll::add(&get(&item1, "children"), Value::Object(item2.clone())).unwrap();
    em.persist(&item1).unwrap();
    em.persist(&item2).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let item3 = find(&em, "DDC531Item", get(&item2, "id")).await.unwrap(); // Load child item first (id 2)
    // parent will already be loaded, cannot be lazy because it has mapped subclasses and we would not
    // know which proxy type to put in.
    assert_instance_of("DDC531Item", &get(&item3, "parent"));
    assert!(!is_uninitialized(&em, &get(&item3, "parent")));
    let item4 = find(&em, "DDC531Item", get(&item1, "id")).await.unwrap(); // Load parent item (id 1)
    assert!(matches!(get(&item4, "parent"), Value::Null));
    assert!(!matches!(get(&item4, "children"), Value::Null));
    assert!(contains(&item4, "children", &item3).await); // lazy-loads children
}
