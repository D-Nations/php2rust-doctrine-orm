//! Ticket/GH12166/GH12166Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12166/GH12166Test.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH12166/LazyEntityWithReadonlyId.php

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

use php_value::Value;

use orm::*;

const LAZY_ENTITY_WITH_READONLY_ID: &str =
    "Doctrine\\Tests\\ORM\\Functional\\Ticket\\GH12166\\LazyEntityWithReadonlyId";

#[tokio::test]
async fn testProxyWithReadonlyIdIsNotInitializedImmediately() {
    let em = orm::set_up(&[LAZY_ENTITY_WITH_READONLY_ID]).await;
    let entity = new_object(LAZY_ENTITY_WITH_READONLY_ID);
    set(&entity, "id", Value::Int(123));
    set(&entity, "name", s("Test Name"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let proxy = em
        .get_reference(LAZY_ENTITY_WITH_READONLY_ID, Value::Int(123))
        .await
        .unwrap()
        .unwrap();
    em.get_class_metadata(LAZY_ENTITY_WITH_READONLY_ID).unwrap();

    assert!(
        em.is_uninitialized_object(&Value::Object(proxy.clone())),
        "Proxy should remain uninitialized after creation"
    );

    let id = get(&proxy, "id");
    assert!(Value::Int(123).identical(&id));
    assert!(
        em.is_uninitialized_object(&Value::Object(proxy.clone())),
        "Proxy should remain uninitialized after accessing ID"
    );
}
