//! Ticket/Issue9300Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the `issue9300` model set).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/Issue9300Test.php

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

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};

use orm::*;

const ISSUE9300_CHILD: &str = "Doctrine\\Tests\\Models\\Issue9300\\Issue9300Child";
const ISSUE9300_PARENT: &str = "Doctrine\\Tests\\Models\\Issue9300\\Issue9300Parent";

async fn set_up() -> std::sync::Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    orm::set_up(&[ISSUE9300_CHILD, ISSUE9300_PARENT]).await
}

/// `new Issue9300Child()`.
fn child() -> php_value::Object {
    let child = new_object(ISSUE9300_CHILD);
    set(
        &child,
        "parents",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    child
}

#[tokio::test]
async fn testPersistedCollectionIsPresentInOriginalDataAfterFlush() {
    let em = set_up().await;
    let parent = new_object(ISSUE9300_PARENT);
    let child = child();
    coll::add(&get(&child, "parents"), Value::Object(parent.clone())).unwrap();

    set(&parent, "name", s("abc"));
    set(&child, "name", s("abc"));

    em.persist(&parent).unwrap();
    em.persist(&child).unwrap();
    em.flush().await.unwrap();

    set(&parent, "name", s("abcd"));
    set(&child, "name", s("abcd"));

    em.flush().await.unwrap();

    assert!(
        em.get_unit_of_work()
            .get_original_entity_data(&child)
            .contains_key(&"parents".into())
    );
}

#[tokio::test]
async fn testPersistingCollectionAfterFlushWorksAsExpected() {
    let em = set_up().await;
    let parent_one = new_object(ISSUE9300_PARENT);
    let parent_two = new_object(ISSUE9300_PARENT);
    let child_one = child();

    set(&parent_one, "name", s("abc"));
    set(&parent_two, "name", s("abc"));
    set(&child_one, "name", s("abc"));
    set(
        &child_one,
        "parents",
        ArrayCollectionObject::value(PhpArray::list(vec![Value::Object(parent_one.clone())])),
    );

    em.persist(&parent_one).unwrap();
    em.persist(&parent_two).unwrap();
    em.persist(&child_one).unwrap();
    em.flush().await.unwrap();

    // Recalculate change-set -> new original data
    set(&child_one, "name", s("abcd"));
    em.flush().await.unwrap();

    set(
        &child_one,
        "parents",
        ArrayCollectionObject::value(PhpArray::list(vec![Value::Object(parent_two.clone())])),
    );
    em.flush().await.unwrap();
    em.clear().unwrap();

    let child_one_fresh = em
        .find(ISSUE9300_CHILD, get(&child_one, "id"))
        .await
        .unwrap()
        .unwrap();
    let parents = get(&child_one_fresh, "parents");
    assert_eq!(1, coll::count(&parents).await.unwrap());
    let first = coll::get(&parents, 0).await.unwrap();
    assert_loose_eq(&get(&parent_two, "id"), &get(&obj(&first), "id"));
}
