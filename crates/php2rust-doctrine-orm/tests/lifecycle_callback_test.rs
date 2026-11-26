//! Upstream: doctrine/orm tests/Tests/ORM/Functional/LifecycleCallbackTest.php
//!
//! The entities' callback bodies are registered below (what a compiler
//! would emit for the PHP methods); a static property is an atomic flag.
//! The event arguments an entity keeps are their class names.

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once};

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::event::{self, event_args_class};
use php2rust_doctrine_orm::object::register_method;

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\";

fn class(name: &str) -> String {
    format!("{NS}{name}")
}

/// `LifecycleCallbackTestEntity::$postLoadCallbackInvoked`.
static ENTITY_POST_LOAD_INVOKED: AtomicBool = AtomicBool::new(false);
/// `LifecycleCallbackCascader::$postLoadCallbackInvoked`.
static CASCADER_POST_LOAD_INVOKED: AtomicBool = AtomicBool::new(false);

fn register_callbacks() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_method(
            &class("LifecycleCallbackTestUser"),
            "testCallback",
            Arc::new(|entity, _| {
                set(entity, "value", s("Hello World"));
                Ok(())
            }),
        );
        let test_entity = class("LifecycleCallbackTestEntity");
        register_method(
            &test_entity,
            "doStuffOnPrePersist",
            Arc::new(|entity, _| {
                set(entity, "prePersistCallbackInvoked", Value::Bool(true));
                Ok(())
            }),
        );
        register_method(
            &test_entity,
            "doStuffOnPostPersist",
            Arc::new(|entity, _| {
                set(entity, "postPersistCallbackInvoked", Value::Bool(true));
                Ok(())
            }),
        );
        register_method(
            &test_entity,
            "doStuffOnPostLoad",
            Arc::new(|entity, _| {
                ENTITY_POST_LOAD_INVOKED.store(true, Ordering::SeqCst);
                let cascader = !get(entity, "cascader").is_null();
                set(entity, "postLoadCascaderNotNull", Value::Bool(cascader));
                Ok(())
            }),
        );
        register_method(
            &test_entity,
            "doStuffOnPreUpdate",
            Arc::new(|entity, _| {
                set(entity, "value", s("changed from preUpdate callback!"));
                Ok(())
            }),
        );
        register_method(
            &test_entity,
            "doStuffOnPreFlush",
            Arc::new(|entity, _| {
                set(entity, "preFlushCallbackInvoked", Value::Bool(true));
                Ok(())
            }),
        );
        register_method(
            &class("LifecycleCallbackCascader"),
            "doStuffOnPostLoad",
            Arc::new(|entity, _| {
                CASCADER_POST_LOAD_INVOKED.store(true, Ordering::SeqCst);
                let count = match get(entity, "entities") {
                    Value::Object(entities) => entities.php_count().and_then(Result::ok).unwrap_or(0),
                    _ => 0,
                };
                set(entity, "postLoadEntitiesCount", Value::Int(count));
                Ok(())
            }),
        );
        register_method(&class("LifecycleCallbackParentEntity"), "doStuff", Arc::new(|_, _| Ok(())));
        for handler in [
            "postPersistHandler",
            "prePersistHandler",
            "postUpdateHandler",
            "preUpdateHandler",
            "postRemoveHandler",
            "preRemoveHandler",
            "preFlushHandler",
            "postLoadHandler",
        ] {
            register_method(
                &class("LifecycleCallbackEventArgEntity"),
                handler,
                Arc::new(move |entity, args| {
                    let mut calls = match get(entity, "calls") {
                        Value::Array(calls) => calls,
                        _ => PhpArray::new(),
                    };
                    calls.insert(handler, s(event_args_class(args).unwrap_or("?")));
                    set(entity, "calls", Value::Array(calls));
                    Ok(())
                }),
            );
        }
    });
}

async fn set_up_test() -> Arc<php2rust_doctrine_orm::entity_manager::EntityManager> {
    register_callbacks();
    set_up(&[
        &class("LifecycleCallbackEventArgEntity"),
        &class("LifecycleCallbackTestEntity"),
        &class("LifecycleCallbackTestUser"),
        &class("LifecycleCallbackCascader"),
    ])
    .await
}

fn test_entity() -> Object {
    new_object(&class("LifecycleCallbackTestEntity"))
}

#[tokio::test]
async fn test_pre_flush_callbacks_are_invoked() {
    let em = set_up_test().await;
    let entity = test_entity();
    set(&entity, "value", s("hello"));
    em.persist(&entity).unwrap();

    em.flush().await.unwrap();

    assert_eq!(get(&entity, "prePersistCallbackInvoked"), Value::Bool(true));
    assert_eq!(get(&entity, "preFlushCallbackInvoked"), Value::Bool(true));

    set(&entity, "preFlushCallbackInvoked", Value::Bool(false));
    em.flush().await.unwrap();

    assert_eq!(get(&entity, "preFlushCallbackInvoked"), Value::Bool(true));

    set(&entity, "value", s("bye"));
    set(&entity, "preFlushCallbackInvoked", Value::Bool(false));
    em.flush().await.unwrap();

    assert_eq!(get(&entity, "preFlushCallbackInvoked"), Value::Bool(true));
}

#[tokio::test]
async fn test_changes_dont_get_lost() {
    let em = set_up_test().await;
    let user = new_object(&class("LifecycleCallbackTestUser"));
    set(&user, "name", s("Bob"));
    set(&user, "value", s("value"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    set(&user, "name", s("Alice"));
    em.flush().await.unwrap(); // Triggers preUpdate

    em.clear().unwrap();

    let user2 = em.find(user.class(), get(&user, "id")).await.unwrap().unwrap();

    assert_eq!(get(&user2, "name"), s("Alice"));
    assert_eq!(get(&user2, "value"), s("Hello World"));
}

#[tokio::test]
async fn test_get_reference_with_post_load_event_is_delayed_until_proxy_trigger() {
    let em = set_up_test().await;
    let entity = test_entity();
    set(&entity, "value", s("hello"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    let id = get(&entity, "id");

    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false, Ordering::SeqCst); // Reset the tracking of the postLoad invocation

    let reference = em
        .get_reference(&class("LifecycleCallbackTestEntity"), id)
        .unwrap()
        .unwrap();
    assert!(
        !reference.properties().contains_key(&"postLoadCallbackInvoked".into()),
        "a static property is not an object property"
    );
    assert!(em.is_uninitialized_object(&Value::Object(reference.clone())));

    // $reference->getValue(): trigger proxy load
    em.initialize_object(&reference).await.unwrap();
    assert!(ENTITY_POST_LOAD_INVOKED.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_post_load_triggered_on_refresh() {
    let em = set_up_test().await;
    let entity = test_entity();
    set(&entity, "value", s("hello"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    let id = get(&entity, "id");

    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false, Ordering::SeqCst); // Reset the tracking of the postLoad invocation

    let reference = em
        .find(&class("LifecycleCallbackTestEntity"), id)
        .await
        .unwrap()
        .unwrap();
    assert!(ENTITY_POST_LOAD_INVOKED.load(Ordering::SeqCst));
    ENTITY_POST_LOAD_INVOKED.store(false, Ordering::SeqCst);

    em.refresh(&reference).await.unwrap();
    assert!(
        ENTITY_POST_LOAD_INVOKED.load(Ordering::SeqCst),
        "postLoad should be invoked when refresh() is called."
    );
}

#[tokio::test]
async fn test_cascaded_entities_calls_pre_persist() {
    let em = set_up_test().await;
    let e1 = test_entity();
    let e2 = test_entity();

    let c = new_object(&class("LifecycleCallbackCascader"));
    set(&c, "entities", ArrayCollectionObject::value(PhpArray::new()));
    em.persist(&c).unwrap();

    append(&c, "entities", Value::Object(e1.clone()));
    append(&c, "entities", Value::Object(e2.clone()));
    set_obj(&e1, "cascader", &c);
    set_obj(&e2, "cascader", &c);

    em.flush().await.unwrap();

    assert_eq!(get(&e1, "prePersistCallbackInvoked"), Value::Bool(true));
    assert_eq!(get(&e2, "prePersistCallbackInvoked"), Value::Bool(true));
}

#[tokio::test]
async fn test_lifecycle_callbacks_get_inherited() {
    let em = set_up_test().await;
    let child_meta = em.get_class_metadata(&class("LifecycleCallbackChildEntity")).unwrap();
    let callbacks: Vec<(String, Vec<String>)> = child_meta
        .read()
        .lifecycle_callbacks
        .iter()
        .map(|(k, v)| (k.to_owned(), v.clone()))
        .collect();
    assert_eq!(callbacks, vec![("prePersist".to_owned(), vec!["doStuff".to_owned()])]);
}

#[tokio::test]
async fn test_lifecycle_callback_event_args() {
    let em = set_up_test().await;
    let e = new_object(&class("LifecycleCallbackEventArgEntity"));

    set(&e, "value", s("foo"));
    em.persist(&e).unwrap();
    em.flush().await.unwrap();

    set(&e, "value", s("var"));
    em.persist(&e).unwrap();
    em.flush().await.unwrap();

    em.refresh(&e).await.unwrap();

    em.remove(&e).await.unwrap();
    em.flush().await.unwrap();

    let Value::Array(calls) = get(&e, "calls") else {
        panic!("calls");
    };
    for (handler, args) in [
        ("preFlushHandler", event::PRE_FLUSH_EVENT_ARGS),
        ("postLoadHandler", event::POST_LOAD_EVENT_ARGS),
        ("prePersistHandler", event::PRE_PERSIST_EVENT_ARGS),
        ("postPersistHandler", event::POST_PERSIST_EVENT_ARGS),
        ("preUpdateHandler", event::PRE_UPDATE_EVENT_ARGS),
        ("postUpdateHandler", event::POST_UPDATE_EVENT_ARGS),
        ("preRemoveHandler", event::PRE_REMOVE_EVENT_ARGS),
        ("postRemoveHandler", event::POST_REMOVE_EVENT_ARGS),
    ] {
        assert_eq!(calls.get(&handler.into()), Some(&s(args)), "{handler}");
    }
}
