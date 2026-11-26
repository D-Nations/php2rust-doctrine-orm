//! Ticket/GH6402Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the quote models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH6402Test.php

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
#[path = "common/quote.rs"]
mod quote;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::*;
use quote::*;

/// `createAddress()`.
async fn create_address(em: &EntityManager) -> Value {
    let address = new_object(QUOTE_ADDRESS);
    set(&address, "zip", s("bar"));

    persist_address(em, &address).await;

    get(&address, "id")
}

/// `createFullAddress()`.
async fn create_full_address(em: &EntityManager) -> Value {
    let address = new_object(QUOTE_FULL_ADDRESS);
    set(&address, "zip", s("bar"));
    // new City('London')
    let city = new_object(QUOTE_CITY);
    set(&city, "name", s("London"));
    set_obj(&address, "city", &city);

    persist_address(em, &address).await;

    get(&address, "id")
}

/// `persistAddress($address)`.
async fn persist_address(em: &EntityManager, address: &Object) {
    let user = quote_user("foo");
    // $user->setAddress($address), which calls $address->setUser($user)
    set_obj(&user, "address", address);
    set_obj(address, "user", &user);

    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();
}

/// `$this->_em->createQuery('SELECT a FROM ' . $class . ' a WHERE a.id = :id')`.
async fn query(em: &Arc<EntityManager>, class: &str, id: Value) -> Vec<Value> {
    let query = em.create_query(&format!("SELECT a FROM {class} a WHERE a.id = :id"));
    query.set_parameter("id", id, None);
    list(&query.get_object_result().await.unwrap())
}

#[tokio::test]
async fn testFind() {
    let em = orm::set_up(&QUOTE_MODELS).await;
    let id = create_address(&em).await;

    let address = em.find(QUOTE_ADDRESS, id).await.unwrap().unwrap();
    assert!(!get(&address, "user").is_null());
}

#[tokio::test]
async fn testQuery() {
    let em = orm::set_up(&QUOTE_MODELS).await;
    let id = create_address(&em).await;

    let addresses = query(&em, QUOTE_ADDRESS, id).await;

    assert_eq!(1, addresses.len());
    assert!(!get(&obj(&addresses[0]), "user").is_null());
}

#[tokio::test]
async fn testFindWithSubClass() {
    let em = orm::set_up(&QUOTE_MODELS).await;
    let id = create_full_address(&em).await;

    let address = em.find(QUOTE_FULL_ADDRESS, id).await.unwrap().unwrap();
    assert!(!get(&address, "user").is_null());
}

#[tokio::test]
async fn testQueryWithSubClass() {
    let em = orm::set_up(&QUOTE_MODELS).await;
    let id = create_full_address(&em).await;

    let addresses = query(&em, QUOTE_FULL_ADDRESS, id).await;

    assert_eq!(1, addresses.len());
    assert!(!get(&obj(&addresses[0]), "user").is_null());
}
