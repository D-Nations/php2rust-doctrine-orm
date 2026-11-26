//! Ticket/GH11163Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH11163Test.php

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
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::*;
use ticket::*;

/// `new GH11163Bucket($id)`.
fn bucket(id: i64) -> Object {
    let bucket = new("GH11163Bucket");
    set(&bucket, "id", Value::Int(id));
    set(
        &bucket,
        "items",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    bucket
}

/// `new GH11163BucketItem($id, $bucket, $position)`.
fn bucket_item(id: i64, bucket: &Object, position: i64) -> Object {
    let item = new("GH11163BucketItem");
    set(&item, "id", Value::Int(id));
    set_obj(&item, "bucket", bucket);
    set(&item, "position", Value::Int(position));
    item
}

#[tokio::test]
async fn testFetchEagerModeWithOrderBy() {
    let em = ticket::set_up(&["GH11163Bucket", "GH11163BucketItem"]).await;

    // Load entities into database
    let bucket = bucket(11163);
    em.persist(&bucket).unwrap();
    em.persist(&bucket_item(1, &bucket, 2)).unwrap();
    em.persist(&bucket_item(2, &bucket, 3)).unwrap();
    em.persist(&bucket_item(3, &bucket, 1)).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    // Fetch entity from database
    let dql = format!(
        "SELECT bucket FROM {} bucket WHERE bucket.id = :id",
        class("GH11163Bucket")
    );
    let query = em.create_query(&dql);
    query.set_parameter("id", Value::Int(11163), None);
    let bucket = query.get_single_result(None).await.unwrap();

    // Assert associated entity is loaded eagerly
    assert_instance_of("GH11163Bucket", &bucket);
    let bucket = obj(&bucket);
    let items = get(&bucket, "items");
    let pc = PersistentCollection::from_value(&items).expect("a PersistentCollection");
    assert!(pc.is_initialized());

    assert_eq!(3, coll::count(&items).await.unwrap());

    // Assert order of entities
    let item = |i: i64| {
        let items = items.clone();
        async move { obj(&coll::get(&items, i).await.unwrap()) }
    };
    assert_eq!(Value::Int(1), get(&item(0).await, "position"));
    assert_eq!(Value::Int(3), get(&item(0).await, "id"));

    assert_eq!(Value::Int(2), get(&item(1).await, "position"));
    assert_eq!(Value::Int(1), get(&item(1).await, "id"));

    assert_eq!(Value::Int(3), get(&item(2).await, "position"));
    assert_eq!(Value::Int(2), get(&item(2).await, "id"));
}
