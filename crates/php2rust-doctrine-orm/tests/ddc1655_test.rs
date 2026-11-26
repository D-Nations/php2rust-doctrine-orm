//! Ticket/DDC1655Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite`. The `#[PostLoad]` methods are registered bodies
//! (`register_method()`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC1655Test.php

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

use std::sync::{Arc, Once};

use php_value::{Object, Value};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::object::register_method;

use orm::{assert_loose_eq, get, new_object, set};

const DDC1655_FOO: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1655Foo";
const DDC1655_BAR: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1655Bar";
const DDC1655_BAZ: &str = "Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1655Baz";

/// `$this->$property++`.
fn increment(entity: &Object, property: &str) {
    let next = match get(entity, property) {
        Value::Int(n) => n + 1,
        _ => 1,
    };
    set(entity, property, Value::Int(next));
}

fn register_callbacks() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_method(
            DDC1655_FOO,
            "postLoad",
            Arc::new(|entity, _| {
                increment(entity, "loaded");
                Ok(())
            }),
        );
        register_method(
            DDC1655_BAR,
            "postSubLoaded",
            Arc::new(|entity, _| {
                increment(entity, "subLoaded");
                Ok(())
            }),
        );
    });
}

async fn set_up() -> Arc<EntityManager> {
    register_callbacks();
    orm::set_up(&[DDC1655_FOO, DDC1655_BAR, DDC1655_BAZ]).await
}

/// `['postLoad' => [...]]`.
fn lifecycle_callbacks(em: &EntityManager, class: &str) -> Vec<(String, Vec<String>)> {
    em.get_class_metadata(class)
        .unwrap()
        .read()
        .lifecycle_callbacks
        .iter()
        .map(|(event, methods)| (event.to_owned(), methods.clone()))
        .collect()
}

#[tokio::test]
async fn testPostLoadOneToManyInheritance() {
    let em = set_up().await;
    assert_eq!(
        vec![("postLoad".to_owned(), vec!["postLoad".to_owned()])],
        lifecycle_callbacks(&em, DDC1655_FOO)
    );

    assert_eq!(
        vec![(
            "postLoad".to_owned(),
            vec!["postLoad".to_owned(), "postSubLoaded".to_owned()]
        )],
        lifecycle_callbacks(&em, DDC1655_BAR)
    );

    let baz = new_object(DDC1655_BAZ);
    let foo = new_object(DDC1655_FOO);
    set(&foo, "baz", Value::Object(baz.clone()));
    let bar = new_object(DDC1655_BAR);
    set(&bar, "baz", Value::Object(baz.clone()));

    em.persist(&foo).unwrap();
    em.persist(&bar).unwrap();
    em.persist(&baz).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let baz = em
        .find(baz.class(), get(&baz, "id"))
        .await
        .unwrap()
        .unwrap();
    let foos = coll::to_array(&get(&baz, "foos")).await.unwrap();
    for foo in foos.values() {
        let Value::Object(foo) = foo else {
            panic!("an entity: {foo:?}");
        };
        let loaded = get(foo, "loaded");
        assert!(
            php_value::compare::loose_equals(&Value::Int(1), &loaded),
            "should have loaded callback counter incremented for {}",
            foo.class()
        );
    }
}

/// Check that post load is not executed several times when the entity
/// is rehydrated again although its already known.
#[tokio::test]
async fn testPostLoadInheritanceChild() {
    let em = set_up().await;
    let bar = new_object(DDC1655_BAR);

    em.persist(&bar).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let bar = em
        .find(bar.class(), get(&bar, "id"))
        .await
        .unwrap()
        .unwrap();
    assert_loose_eq(&Value::Int(1), &get(&bar, "loaded"));
    assert_loose_eq(&Value::Int(1), &get(&bar, "subLoaded"));

    let bar = em
        .find(bar.class(), get(&bar, "id"))
        .await
        .unwrap()
        .unwrap();
    assert_loose_eq(&Value::Int(1), &get(&bar, "loaded"));
    assert_loose_eq(&Value::Int(1), &get(&bar, "subLoaded"));

    let dql =
        "SELECT b FROM Doctrine\\Tests\\ORM\\Functional\\Ticket\\DDC1655Bar b WHERE b.id = ?1";
    let q = em.create_query(dql);
    q.set_parameter(1, get(&bar, "id"), None);
    let bar = orm::obj(&q.get_single_result(None).await.unwrap());

    assert_loose_eq(&Value::Int(1), &get(&bar, "loaded"));
    assert_loose_eq(&Value::Int(1), &get(&bar, "subLoaded"));

    em.refresh(&bar).await.unwrap();

    assert_loose_eq(&Value::Int(2), &get(&bar, "loaded"));
    assert_loose_eq(&Value::Int(2), &get(&bar, "subLoaded"));
}
