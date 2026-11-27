//! Port of `CascadeRemoveOrderTest`, on `SQLite`. The models' constructors
//! are written here, on their recorded classes.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/CascadeRemoveOrderTest.php

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

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{append, get, new_object, set};

const ENTITY_O: &str = "Doctrine\\Tests\\ORM\\Functional\\CascadeRemoveOrderEntityO";
const ENTITY_G: &str = "Doctrine\\Tests\\ORM\\Functional\\CascadeRemoveOrderEntityG";

/// `new CascadeRemoveOrderEntityO()`.
fn entity_o() -> Object {
    let entity = new_object(ENTITY_O);
    set(
        &entity,
        "oneToManyG",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    entity
}

/// `new CascadeRemoveOrderEntityG($eO)`.
fn entity_g(owner: &Object) -> Object {
    let entity = new_object(ENTITY_G);
    set(&entity, "ownerO", Value::Object(owner.clone()));
    set(&entity, "position", Value::Int(1));
    // $this->ownerO->addOneToManyG($this);
    append(owner, "oneToManyG", Value::Object(entity.clone()));
    entity
}

async fn set_up() -> Arc<EntityManager> {
    orm::set_up(&[ENTITY_O, ENTITY_G]).await
}

#[tokio::test]
async fn testSingle() {
    let em = set_up().await;
    let e_o = entity_o();
    let e_g = entity_g(&e_o);

    em.persist(&e_o).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let e_o_loaded = em.find(ENTITY_O, get(&e_o, "id")).await.unwrap().unwrap();

    em.remove(&e_o_loaded).await.unwrap();
    em.flush().await.unwrap();

    assert!(em.find(ENTITY_G, get(&e_g, "id")).await.unwrap().is_none());
}

#[tokio::test]
async fn testMany() {
    let em = set_up().await;
    let e_o = entity_o();
    let e_g1 = entity_g(&e_o);
    let e_g2 = entity_g(&e_o);
    let e_g3 = entity_g(&e_o);

    set(&e_o, "oneToOneG", Value::Object(e_g2.clone()));

    em.persist(&e_o).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let e_o_loaded = em.find(ENTITY_O, get(&e_o, "id")).await.unwrap().unwrap();

    em.remove(&e_o_loaded).await.unwrap();
    em.flush().await.unwrap();

    assert!(em.find(ENTITY_G, get(&e_g1, "id")).await.unwrap().is_none());
    assert!(em.find(ENTITY_G, get(&e_g2, "id")).await.unwrap().is_none());
    assert!(em.find(ENTITY_G, get(&e_g3, "id")).await.unwrap().is_none());
}
