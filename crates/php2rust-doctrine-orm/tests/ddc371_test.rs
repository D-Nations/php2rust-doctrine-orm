//! Ticket/DDC371Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC371Test.php

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
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;
use php2rust_doctrine_orm::query::context::{HINT_REFRESH, Hint};

use orm::*;
use ticket::*;

#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&["DDC371Parent", "DDC371Child"]).await;

    let parent = new("DDC371Parent");
    set(&parent, "data", s("parent"));
    set(
        &parent,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );

    let child = new("DDC371Child");
    set(&child, "data", s("child"));

    set_obj(&child, "parent", &parent);
    coll::add(&get(&parent, "children"), Value::Object(child.clone())).unwrap();

    em.persist(&parent).unwrap();
    em.persist(&child).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!(
        "select c,p from {NS}DDC371Child c left join c.parent p where c.id = 1 and p.id = 1"
    ));
    query.set_hint(HINT_REFRESH, Hint::Value(Value::Bool(true)));
    let children = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());

    assert_eq!(1, children.len());
    let child_parent = get(&obj(&children[0]), "parent");
    assert!(!is_uninitialized(&em, &child_parent));
    let collection =
        PersistentCollection::from_value(&get(&obj(&child_parent), "children")).unwrap();
    assert!(!collection.is_initialized());
    assert_eq!(0, collection.unwrap().len());
}
