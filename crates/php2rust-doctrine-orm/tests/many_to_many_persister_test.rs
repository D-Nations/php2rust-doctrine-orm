//! Port of `ManyToManyPersisterTest`. The connection is a `SQLite` one
//! with the schema and its statements logged (`tests/common/query_log.rs`)
//! in place of the mocked `executeStatement()`: the `DELETE` the persister
//! issues is the one logged statement after the log is reset.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Persisters/ManyToManyPersisterTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::persisters::collection;

use orm::{append, get, new_object, set};

const NS: &str = "Doctrine\\Tests\\Models\\ManyToManyPersister\\";

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// `new ParentClass($id)`.
fn parent_class(id: i64) -> Object {
    let parent = new_object(&class("ParentClass"));
    set(&parent, "id", Value::Int(id));
    set(
        &parent,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    parent
}

/// `new OtherParentClass($id)`.
fn other_parent_class(id: i64) -> Object {
    let other = new_object(&class("OtherParentClass"));
    set(&other, "id", Value::Int(id));
    other
}

/// `new ChildClass($id1, $otherParent)`.
fn child_class(id1: i64, other_parent: &Object) -> Object {
    let child = new_object(&class("ChildClass"));
    set(&child, "id1", Value::Int(id1));
    set(&child, "otherParent", Value::Object(other_parent.clone()));
    set(
        &child,
        "parents",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    child
}

#[tokio::test]
async fn testDeleteManyToManyCollection() {
    let (em, log) = query_log::set_up_logged(&[
        &class("ParentClass"),
        &class("OtherParentClass"),
        &class("ChildClass"),
    ])
    .await;

    let parent = parent_class(1);
    let other_parent = other_parent_class(42);
    let child = child_class(1, &other_parent);

    append(&parent, "children", Value::Object(child.clone()));
    append(&child, "parents", Value::Object(parent.clone()));

    em.persist(&parent).unwrap();
    em.flush().await.unwrap();

    let mut id = PhpArray::new();
    id.insert("id1", Value::Int(1));
    id.insert("otherParent", Value::Object(other_parent.clone()));
    let child_reloaded = em
        .find(&class("ChildClass"), Value::Array(id))
        .await
        .unwrap()
        .expect("a ChildClass");
    assert_eq!(child_reloaded.class(), class("ChildClass"));

    log.reset_and_enable();

    let parents = PersistentCollection::from_value(&get(&child_reloaded, "parents"))
        .expect("a persistent collection");
    collection::delete(&em, &parents).await.unwrap();

    let entries = log.entries();
    assert_eq!(entries.len(), 1, "{entries:#?}");
    assert_eq!(
        entries[0].0,
        "DELETE FROM parent_child WHERE child_id1 = ? AND child_id2 = ?"
    );
    let params: Vec<Value> = entries[0].1.iter().map(|(_, v)| v.clone()).collect();
    assert_eq!(params, vec![Value::Int(1), Value::Int(42)]);
}
