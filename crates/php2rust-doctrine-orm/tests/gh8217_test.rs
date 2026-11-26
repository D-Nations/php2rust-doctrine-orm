//! Ticket/GH8217Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models), with a logging connection (`tests/common/query_log.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8217Test.php

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

#[path = "common/query_log.rs"]
mod query_log;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;
use ticket::*;

/// `new GH8217CollectionItem($collection, $collectionIndex)` and
/// `$collection->addItem()`.
fn add_item(collection: &Object, collection_index: i64) {
    let item = new("GH8217CollectionItem");
    set_obj(&item, "collection", collection);
    set(&item, "collectionIndex", Value::Int(collection_index));
    coll::add(&get(collection, "items"), Value::Object(item)).unwrap();
}

#[tokio::test]
async fn testNoQueriesAfterSecondFlush() {
    let classes = [class("GH8217Collection"), class("GH8217CollectionItem")];
    let classes: Vec<&str> = classes.iter().map(String::as_str).collect();
    let (em, log) = query_log::set_up_logged(&classes).await;

    let collection = new("GH8217Collection");
    set(
        &collection,
        "items",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    add_item(&collection, 0);
    add_item(&collection, 1);
    em.persist(&collection).unwrap();
    em.flush().await.unwrap();

    log.reset_and_enable();
    em.flush().await.unwrap();
    assert_eq!(0, log.queries().len());
}
