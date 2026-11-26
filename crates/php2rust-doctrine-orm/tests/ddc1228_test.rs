//! Ticket/DDC1228Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//! A method call on a lazy proxy initializes it: the tests call
//! `initialize_object()` where PHP calls a method.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1228Test.php

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

use php_value::{Value, php_array};

use orm::*;

const USER: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1228User";
const PROFILE: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1228Profile";

/// `new DDC1228User()`: `$name = 'Bar'`.
fn user() -> php_value::Object {
    new_object(USER)
}

#[tokio::test]
async fn testOneToOnePersist() {
    let em = orm::set_up(&[USER, PROFILE]).await;

    let user = user();
    let profile = new_object(PROFILE);
    set(&profile, "name", s("Foo"));
    set_obj(&user, "profile", &profile);

    em.persist(&user).unwrap();
    em.persist(&profile).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(USER, get(&user, "id")).await.unwrap().unwrap();

    let profile = get_object(&user, "profile").unwrap();
    assert!(
        em.is_uninitialized_object(&Value::Object(profile.clone())),
        "Proxy is not initialized"
    );
    // $user->getProfile()->setName('Bar')
    em.initialize_object(&profile).await.unwrap();
    set(&profile, "name", s("Bar"));
    assert!(
        !em.is_uninitialized_object(&Value::Object(profile.clone())),
        "Proxy is not initialized"
    );

    assert_eq!(s("Bar"), get(&profile, "name"));
    assert_loose_eq(
        &Value::Array(php_array! { "id" => Value::Int(1), "name" => s("Foo") }),
        &Value::Array(em.get_unit_of_work().get_original_entity_data(&profile)),
    );

    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(USER, get(&user, "id")).await.unwrap().unwrap();
    let profile = get_object(&user, "profile").unwrap();
    em.initialize_object(&profile).await.unwrap();
    assert_eq!(s("Bar"), get(&profile, "name"));
}

#[tokio::test]
async fn testRefresh() {
    let em = orm::set_up(&[USER, PROFILE]).await;

    let user = user();
    let profile = new_object(PROFILE);
    set(&profile, "name", s("Foo"));
    set_obj(&user, "profile", &profile);

    em.persist(&user).unwrap();
    em.persist(&profile).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em
        .get_reference(USER, get(&user, "id"))
        .await
        .unwrap()
        .unwrap();
    em.refresh(&user).await.unwrap();
    set(&user, "name", s("Baz"));
    em.flush().await.unwrap();
    em.clear().unwrap();

    let user = em.find(USER, get(&user, "id")).await.unwrap().unwrap();
    assert_eq!(s("Baz"), get(&user, "name"));
}
