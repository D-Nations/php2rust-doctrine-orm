//! Port of `EagerFetchCollectionTest`, on `SQLite`. The query count is the
//! length of the connection's query log.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/EagerFetchCollectionTest.php

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

use std::sync::Arc;

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::class_metadata as cm;
use php2rust_doctrine_orm::query::exec::SqlStatements;
use php2rust_doctrine_orm::tools::pagination::{OffsetPaginator, Page, Window};

use query_log::QueryLog;

const OWNER: &str = "Doctrine\\Tests\\ORM\\Functional\\EagerFetchOwner";
const CHILD: &str = "Doctrine\\Tests\\ORM\\Functional\\EagerFetchChild";

/// `setUp()`.
async fn set_up() -> (Arc<EntityManager>, Arc<QueryLog>) {
    models::register();
    let (em, log) = query_log::set_up_logged(&[OWNER, CHILD]).await;

    // Ensure tables are empty
    for class in [CHILD, OWNER] {
        em.get_repository(class)
            .unwrap()
            .create_query_builder("o", None)
            .unwrap()
            .delete(None, None)
            .unwrap()
            .get_query()
            .unwrap()
            .execute(None, None)
            .await
            .unwrap();
    }
    log.reset_and_enable();
    (em, log)
}

fn query_count(log: &QueryLog) -> usize {
    log.queries().len()
}

/// `new EagerFetchOwner()`.
fn new_owner() -> Object {
    let owner = orm::new_object(OWNER);
    orm::set(
        &owner,
        "children",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    owner
}

/// `createOwnerWithChildren()`.
fn create_owner_with_children(em: &EntityManager, children: usize) -> Object {
    let owner = new_owner();
    em.persist(&owner).unwrap();

    for _ in 0..children {
        let child = orm::new_object(CHILD);
        orm::set_obj(&child, "owner", &owner);

        orm::append(&owner, "children", Value::Object(child.clone()));

        em.persist(&child).unwrap();
    }

    owner
}

async fn find_owner(em: &EntityManager, id: Value) -> Object {
    em.find(OWNER, id).await.unwrap().unwrap()
}

#[tokio::test]
async fn testEagerFetchMode() {
    let (em, log) = set_up().await;
    let owner = create_owner_with_children(&em, 2);
    let owner2 = create_owner_with_children(&em, 3);

    em.flush().await.unwrap();
    em.clear().unwrap();

    let owner = find_owner(&em, orm::get(&owner, "id")).await;

    let after_query_count = query_count(&log);
    assert_eq!(2, orm::count_of(&owner, "children").await);

    assert_eq!(
        after_query_count,
        query_count(&log),
        "The $owner->children collection should already be initialized by find EagerFetchOwner before."
    );

    let owner2 = find_owner(&em, orm::get(&owner2, "id")).await;
    assert_eq!(3, orm::count_of(&owner2, "children").await);

    em.clear().unwrap();

    let before_query_count = query_count(&log);
    let all_owners = em.get_repository(OWNER).unwrap().find_all().await.unwrap();

    assert_eq!(
        before_query_count + 2,
        query_count(&log),
        "the findAll() + 1 subselect loading both collections of the two returned $owners"
    );

    assert_eq!(2, orm::count_of(&all_owners[0], "children").await);
    assert_eq!(3, orm::count_of(&all_owners[1], "children").await);

    assert_eq!(
        before_query_count + 2,
        query_count(&log),
        "both collections are already initialized and counting them does not make a difference in total query count"
    );
}

#[tokio::test]
async fn testEagerFetchModeWithDQL() {
    let (em, log) = set_up().await;
    create_owner_with_children(&em, 2);
    create_owner_with_children(&em, 3);

    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(&format!("SELECT o FROM {OWNER} o"));
    query.set_fetch_mode(OWNER, "children", cm::FETCH_EAGER);

    let before_query_count = query_count(&log);
    let Value::Array(owners) = query.get_result(HYDRATE_OBJECT).await.unwrap() else {
        panic!("a list of owners");
    };
    let after_query_count = query_count(&log);

    assert_eq!(before_query_count + 2, after_query_count);

    let owners: Vec<Object> = owners
        .values()
        .map(|owner| owner.as_object().unwrap().clone())
        .collect();
    orm::count_of(&owners[0], "children").await;
    orm::count_of(&owners[1], "children").await;

    let another_query_count = query_count(&log);

    assert_eq!(another_query_count, after_query_count);
}

#[tokio::test]
async fn testSubselectFetchJoinWithNotAllowed() {
    let (em, _log) = set_up().await;
    let query = em.create_query(&format!(
        "SELECT o, c FROM {OWNER} o JOIN o.children c WITH c.id = 1"
    ));
    let e = query.get_result(HYDRATE_OBJECT).await.unwrap_err();
    assert_eq!("Doctrine\\ORM\\Query\\QueryException", e.class);
    assert!(
        String::from_utf8_lossy(e.message.as_bytes())
            .contains("Associations with fetch-mode=EAGER may not be using WITH conditions"),
        "{e:?}"
    );
}

#[tokio::test]
async fn testSubselectFetchJoinWithAllowedWhenOverriddenNotEager() {
    let (em, _log) = set_up().await;
    let query = em.create_query(&format!(
        "SELECT o, c FROM {OWNER} o JOIN o.children c WITH c.id = 1"
    ));
    query.set_fetch_mode(CHILD, "owner", cm::FETCH_LAZY);

    assert!(matches!(query.get_sql().unwrap(), SqlStatements::One(_)));
}

#[tokio::test]
async fn testSubselectFetchJoinWithAllowedWhenOverriddenNotEagerPaginator() {
    let (em, _log) = set_up().await;
    let query = em.create_query(&format!(
        "SELECT o, c FROM {OWNER} o JOIN o.children c WITH c.id = 1"
    ));
    query.set_fetch_mode(CHILD, "owner", cm::FETCH_LAZY);

    let page = OffsetPaginator::new(true, None)
        .paginate_window((&query).into(), Window::new(0, 1).unwrap())
        .await
        .unwrap();
    // assertIsArray($page->getItems())
    let _: &[Value] = page.get_items();
}

#[tokio::test]
async fn testEagerFetchWithIterable() {
    let (em, _log) = set_up().await;
    create_owner_with_children(&em, 2);
    em.flush().await.unwrap();
    em.clear().unwrap();

    let iterable = em
        .get_repository(OWNER)
        .unwrap()
        .create_query_builder("o", None)
        .unwrap()
        .get_query()
        .unwrap()
        .to_iterable(None, HYDRATE_OBJECT)
        .await
        .unwrap();

    // There is only a single record, but use a foreach to ensure the iterator is marked as finished and the table lock is released
    for owner in iterable {
        assert_eq!(
            2,
            orm::count_of(owner.as_object().unwrap(), "children").await
        );
    }
}
