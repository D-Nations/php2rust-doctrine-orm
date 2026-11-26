//! Ticket/GH5998Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH5998Test.php

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

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_dbal::LockMode;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use ticket::*;

/// Verifies that `MappedSuperclasses` work within an inheritance hierarchy.
#[tokio::test]
async fn testIssue() {
    let em = ticket::set_up(&[
        "GH5998JTI",
        "GH5998JTIChild",
        "GH5998STI",
        "GH5998Basic",
        "GH5998Related",
    ])
    .await;

    // Test JTI
    class_tests(&em, "GH5998JTIChild").await;
    // Test STI
    class_tests(&em, "GH5998STIChild").await;
    // Test Basic
    class_tests(&em, "GH5998Basic").await;
}

async fn class_tests(em: &Arc<EntityManager>, short: &str) {
    let class_name = class(short);

    // Test insert
    // new $className('Sam', 0, 1)
    let child = new(short);
    set(&child, "firstName", s("Sam"));
    set(&child, "type", Value::Int(0));
    set(&child, "status", Value::Int(1));
    set_obj(&child, "rel", &new("GH5998Related"));
    em.persist(&child).unwrap();
    em.persist(&obj(&get(&child, "rel"))).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
    let id = get(&child, "id");

    // Test find by rel
    let mut criteria = PhpArray::new();
    criteria.insert("rel", get(&child, "rel"));
    let found = em
        .get_repository(&class_name)
        .unwrap()
        .find_one_by(&criteria, None)
        .await
        .unwrap();
    assert!(found.is_some());
    em.clear().unwrap();

    // Test query by id with fetch join
    let query = em.create_query(&format!(
        "SELECT t, r FROM {class_name} t JOIN t.rel r WHERE t.id = ?0"
    ));
    query.set_parameter(0, id.clone(), None);
    let child = query.get_one_or_null_result(None).await.unwrap();
    assert!(!child.is_null());
    let child = obj(&child);

    // Test lock and update
    em.wrap_in_transaction(|em| async move {
        em.lock(&child, LockMode::None, None).await?;
        set(&child, "firstName", s("Bob"));
        set(&child, "status", Value::Int(0));
        Ok(())
    })
    .await
    .unwrap();
    em.clear().unwrap();
    let child = em
        .get_repository(&class_name)
        .unwrap()
        .find(id.clone())
        .await
        .unwrap();
    let child = child.expect("not null");
    assert_loose_eq(&get(&child, "firstName"), &s("Bob"));
    assert_loose_eq(&get(&child, "status"), &Value::Int(0));

    // Test delete
    em.remove(&child).await.unwrap();
    em.flush().await.unwrap();
    let child = em
        .get_repository(&class_name)
        .unwrap()
        .find(id)
        .await
        .unwrap();
    assert!(child.is_none());
}
