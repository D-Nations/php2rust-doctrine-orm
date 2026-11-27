//! Ticket/DDC6460Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC6460Test.php

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

use php_value::Value;
use php2rust_doctrine_orm::entity_manager::EntityManager;

use orm::{assert_loose_eq, get, get_object, s, set};
use ticket::*;

/// `setUp()`: the schema (upstream ignores a `SchemaException`).
async fn set_up_schema() -> Arc<EntityManager> {
    set_up(&["DDC6460Entity", "DDC6460ParentEntity"]).await
}

#[tokio::test]
async fn testInlineEmbeddable() {
    let em = set_up_schema().await;
    let is_field_mapped = em
        .get_class_metadata(&class("DDC6460Entity"))
        .unwrap()
        .read()
        .has_field("embedded");

    assert!(is_field_mapped);
}

#[tokio::test]
async fn testInlineEmbeddableProxyInitialization() {
    let em = set_up_schema().await;

    let entity = new("DDC6460Entity");
    set(&entity, "id", Value::Int(1));
    let embedded = new("DDC6460Embeddable");
    set(&embedded, "field", s("test"));
    set(&entity, "embedded", Value::Object(embedded));
    em.persist(&entity).unwrap();

    let second = new("DDC6460ParentEntity");
    set(&second, "id", Value::Int(1));
    set(&second, "lazyLoaded", Value::Object(entity.clone()));
    em.persist(&second).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();

    let second_entity_with_lazy_parameter = em
        .get_repository(&class("DDC6460ParentEntity"))
        .unwrap()
        .call("findOneById", vec![Value::Int(1)])
        .await
        .unwrap();
    let second_entity_with_lazy_parameter = orm::obj(&second_entity_with_lazy_parameter);

    let lazy_loaded = get(&second_entity_with_lazy_parameter, "lazyLoaded");
    assert_instance_of("DDC6460Entity", &lazy_loaded);
    assert!(is_uninitialized(&em, &lazy_loaded));

    // Reading `->embedded` initializes the proxy.
    let proxy = get_object(&second_entity_with_lazy_parameter, "lazyLoaded").unwrap();
    em.initialize_object(&proxy).await.unwrap();
    assert_loose_eq(&get(&proxy, "embedded"), &get(&entity, "embedded"));
    assert!(!is_uninitialized(&em, &lazy_loaded));
}
