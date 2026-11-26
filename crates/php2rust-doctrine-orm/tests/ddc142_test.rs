//! Ticket/DDC142Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the quote models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC142Test.php

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

use php_value::{Object, Value};

use orm::*;
use quote::*;

/// `$user->setAddress($address)`, which calls `$address->setUser($user)`.
fn set_address(user: &Object, address: &Object) {
    if !same_value(&get(user, "address"), address) {
        set_obj(user, "address", address);
        if !same_value(&get(address, "user"), user) {
            set_obj(address, "user", user);
        }
    }
}

#[tokio::test]
async fn testCreateRetrieveUpdateDelete() {
    let em = orm::set_up(&QUOTE_MODELS).await;

    let user = quote_user("FabioBatSilva");
    em.persist(&user).unwrap();

    let address = new_object(QUOTE_ADDRESS);
    set(&address, "zip", s("12345"));
    em.persist(&address).unwrap();

    em.flush().await.unwrap();

    let address_ref = em
        .get_reference(QUOTE_ADDRESS, get(&address, "id"))
        .await
        .unwrap()
        .unwrap();

    set_address(&user, &address_ref);

    em.flush().await.unwrap();
    em.clear().unwrap();

    let id = get(&user, "id");

    assert_ne!(Value::Null, id);

    let user = em.find(QUOTE_USER, id.clone()).await.unwrap().unwrap();
    let address = get_object(&user, "address").unwrap();

    assert_eq!(QUOTE_USER, user.class());
    assert_eq!(QUOTE_ADDRESS, address.class());
    assert_loose_eq(&s("FabioBatSilva"), &get(&user, "name"));
    assert_loose_eq(&s("12345"), &get(&address, "zip"));

    set(&user, "name", s("FabioBatSilva1"));
    set(&user, "address", Value::Null);

    em.persist(&user).unwrap();
    em.remove(&address).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(QUOTE_USER, id.clone()).await.unwrap().unwrap();
    assert_eq!(QUOTE_USER, user.class());
    assert_eq!(Value::Null, get(&user, "address"));

    assert_loose_eq(&s("FabioBatSilva1"), &get(&user, "name"));

    em.remove(&user).await.unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert!(em.find(QUOTE_USER, id).await.unwrap().is_none());
}
