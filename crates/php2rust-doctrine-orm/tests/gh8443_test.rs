//! Ticket/GH8443Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the `company` model set and its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/GH8443Test.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/company.rs"]
mod company;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/ticket.rs"]
mod ticket;

use std::sync::Arc;

use php_value::{Object, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::query::context::{HINT_FORCE_PARTIAL_LOAD, Hint};

use company::*;
use orm::*;
use ticket::*;

async fn set_up() -> Arc<EntityManager> {
    register_listeners();
    let mut classes = COMPANY_MODELS.to_vec();
    let foo = class("GH8443Foo");
    classes.push(&foo);
    orm::set_up(&classes).await
}

#[tokio::test]
async fn testJoinRootEntityWithForcePartialLoad() {
    let em = set_up().await;
    let person = company_person("John");
    let manager = company_manager("Adam", 1000, "IT", "manager");
    set_spouse(&manager, &person);

    em.persist(&person).unwrap();
    em.persist(&manager).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let query = em.create_query(
        "SELECT m from Doctrine\\Tests\\Models\\Company\\CompanyManager m
               JOIN m.spouse s
               WITH s.name = 'John'",
    );
    query.set_hint(HINT_FORCE_PARTIAL_LOAD, Hint::Value(Value::Bool(true)));
    let manager = obj(&query.get_single_result(None).await.unwrap());
    em.refresh(&manager).await.unwrap();

    let spouse = get_object(&manager, "spouse").unwrap();
    em.initialize_object(&spouse).await.unwrap();
    assert_loose_eq(&s("John"), &get(&spouse, "name"));
}

/// `$foo->setBar($bar)`.
fn set_bar(foo: &Object, bar: &Object) {
    if !get_object(foo, "bar").is_some_and(|current| same(&current, bar)) {
        set_obj(foo, "bar", bar);
        set_obj(bar, "bar", foo);
    }
}

/// `new GH8443Foo($name)`.
fn gh8443_foo(name: &str) -> Object {
    let foo = new("GH8443Foo");
    set(&foo, "name", s(name));
    foo
}

#[tokio::test]
async fn testJoinRootEntityWithOnlyOneEntityInHierarchy() {
    let em = set_up().await;
    let bar = gh8443_foo("bar");
    let foo = gh8443_foo("foo");
    set_bar(&foo, &bar);

    em.persist(&bar).unwrap();
    em.persist(&foo).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let foo = obj(&em
        .create_query(&format!(
            "SELECT f from {} f JOIN f.bar b WITH b.name = 'bar'",
            class("GH8443Foo")
        ))
        .get_single_result(None)
        .await
        .unwrap());
    assert_instance_of("GH8443Foo", &Value::Object(foo.clone()));

    let bar = get_object(&foo, "bar").unwrap();
    em.initialize_object(&bar).await.unwrap();
    assert_loose_eq(&s("bar"), &get(&bar, "name"));
}
