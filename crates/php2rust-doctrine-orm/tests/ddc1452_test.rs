//! Ticket/DDC1452Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models and its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1452Test.php

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

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::abstract_query::HYDRATE_OBJECT;
use php2rust_doctrine_orm::collection_object::{self as coll, ArrayCollectionObject};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;

const ENTITY_A: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1452EntityA";
const ENTITY_B: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1452EntityB";

async fn set_up() -> Arc<EntityManager> {
    let em = orm::set_up(&CMS_MODELS).await;
    create_schema(&em, &[ENTITY_A, ENTITY_B]).await;
    em
}

/// `new DDC1452EntityA()`.
fn entity_a(title: &str) -> php_value::Object {
    let a = new_object(ENTITY_A);
    set(
        &a,
        "entitiesB",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    set(&a, "title", s(title));
    a
}

fn is_uninitialized(em: &EntityManager, value: &Value) -> bool {
    em.is_uninitialized_object(value)
}

#[tokio::test]
async fn testIssue() {
    let em = set_up().await;

    let a1 = entity_a("foo");
    let a2 = entity_a("bar");

    let b = new_object(ENTITY_B);
    set_obj(&b, "entityAFrom", &a1);
    set_obj(&b, "entityATo", &a2);

    em.persist(&a1).unwrap();
    em.persist(&a2).unwrap();
    em.persist(&b).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = format!(
        "SELECT a, b, ba FROM {ENTITY_A} AS a LEFT JOIN a.entitiesB AS b LEFT JOIN b.entityATo AS ba ORDER BY a.id"
    );
    let query = em.create_query(&dql);
    query.set_max_results(Some(1));
    let results = query.get_result(HYDRATE_OBJECT).await.unwrap();

    let first = obj(&at(&results, 0));
    let b0 = obj(&coll::get(&get(&first, "entitiesB"), 0).await.unwrap());
    assert!(same_value(&get(&b0, "entityAFrom"), &first));
    let entity_a_to = get(&b0, "entityATo");
    assert!(!is_uninitialized(&em, &entity_a_to));
    assert!(
        obj(&get(&obj(&entity_a_to), "entitiesB"))
            .instance_of("Doctrine\\Common\\Collections\\Collection")
    );
}

#[tokio::test]
async fn testFetchJoinOneToOneFromInverse() {
    let em = set_up().await;

    let address = cms_address("Germany", "12345", "Bonn");
    set(&address, "street", s("Somestreet"));
    set(&address, "zip", Value::Int(12345));

    let user = cms_user_with("beberlei", "beberlei", "active");
    set_obj(&user, "address", &address);
    set_obj(&address, "user", &user);

    em.persist(&address).unwrap();
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = "SELECT a, u FROM Doctrine\\Tests\\Models\\CMS\\CmsAddress a INNER JOIN a.user u";
    let data = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();
    em.clear().unwrap();

    assert!(!is_uninitialized(&em, &get(&obj(&at(&data, 0)), "user")));

    let dql = "SELECT u, a FROM Doctrine\\Tests\\Models\\CMS\\CmsUser u INNER JOIN u.address a";
    let data = em
        .create_query(dql)
        .get_result(HYDRATE_OBJECT)
        .await
        .unwrap();

    assert!(!is_uninitialized(&em, &get(&obj(&at(&data, 0)), "address")));
}
