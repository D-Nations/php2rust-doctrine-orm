//! Upstream: doctrine/orm tests/Tests/ORM/Functional/LifecycleCallbackTest.php
//!
//! The entities' callback bodies are registered below (what a compiler
//! would emit for the PHP methods); a static property is a thread-local
//! flag (each test runs on its own thread, as PHP runs one test at a time).
//! The event arguments an entity keeps are their class names.
//! `LifecycleCallbackTestUser::testCallback()` is a callback of the model,
//! not a test. Pending: `testCascadedEntitiesLoadedInPostLoad` (the
//! cascader's `postLoad()` counts its uninitialized collection, which a
//! callback body, being synchronous, cannot load).

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::any::Any;
use std::cell::Cell;
use std::sync::{Arc, Once};

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry as _, ListenerError,
};
use php2rust_doctrine_orm::collection_object::ArrayCollectionObject;
use php2rust_doctrine_orm::event::{self, PreUpdateEventArgs, event_args_class};
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::object::register_method;
use php2rust_doctrine_orm::query::context::{HYDRATE_OBJECT, HYDRATE_SIMPLEOBJECT};
use php2rust_doctrine_orm::utility::to_text;

use orm::*;

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\";

fn class(name: &str) -> String {
    format!("{NS}{name}")
}

/// A static `bool` property.
struct StaticFlag(&'static std::thread::LocalKey<Cell<bool>>);

impl StaticFlag {
    fn store(&self, value: bool) {
        self.0.with(|flag| flag.set(value));
    }

    fn load(&self) -> bool {
        self.0.with(Cell::get)
    }
}

thread_local! {
    static ENTITY_POST_LOAD: Cell<bool> = const { Cell::new(false) };
    static CASCADER_POST_LOAD: Cell<bool> = const { Cell::new(false) };
}

/// `LifecycleCallbackTestEntity::$postLoadCallbackInvoked`.
static ENTITY_POST_LOAD_INVOKED: StaticFlag = StaticFlag(&ENTITY_POST_LOAD);
/// `LifecycleCallbackCascader::$postLoadCallbackInvoked`.
static CASCADER_POST_LOAD_INVOKED: StaticFlag = StaticFlag(&CASCADER_POST_LOAD);

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
                ENTITY_POST_LOAD_INVOKED.store(true);
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
                CASCADER_POST_LOAD_INVOKED.store(true);
                let count = match get(entity, "entities") {
                    Value::Object(entities) => {
                        entities.php_count().and_then(Result::ok).unwrap_or(0)
                    }
                    _ => 0,
                };
                set(entity, "postLoadEntitiesCount", Value::Int(count));
                Ok(())
            }),
        );
        register_method(
            &class("LifecycleCallbackParentEntity"),
            "doStuff",
            Arc::new(|_, _| Ok(())),
        );
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

fn new_test_entity() -> Object {
    new_object(&class("LifecycleCallbackTestEntity"))
}

#[tokio::test]
async fn testPreFlushCallbacksAreInvoked() {
    let em = set_up_test().await;
    let entity = new_test_entity();
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
async fn testChangesDontGetLost() {
    let em = set_up_test().await;
    let user = new_object(&class("LifecycleCallbackTestUser"));
    set(&user, "name", s("Bob"));
    set(&user, "value", s("value"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    set(&user, "name", s("Alice"));
    em.flush().await.unwrap(); // Triggers preUpdate

    em.clear().unwrap();

    let user2 = em
        .find(user.class(), get(&user, "id"))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(get(&user2, "name"), s("Alice"));
    assert_eq!(get(&user2, "value"), s("Hello World"));
}

#[tokio::test]
async fn testGetReferenceWithPostLoadEventIsDelayedUntilProxyTrigger() {
    let em = set_up_test().await;
    let entity = new_test_entity();
    set(&entity, "value", s("hello"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    let id = get(&entity, "id");

    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false); // Reset the tracking of the postLoad invocation

    let reference = em
        .get_reference(&class("LifecycleCallbackTestEntity"), id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !reference
            .properties()
            .contains_key(&"postLoadCallbackInvoked".into()),
        "a static property is not an object property"
    );
    assert!(em.is_uninitialized_object(&Value::Object(reference.clone())));

    // $reference->getValue(): trigger proxy load
    em.initialize_object(&reference).await.unwrap();
    assert!(ENTITY_POST_LOAD_INVOKED.load());
}

#[tokio::test]
async fn testPostLoadTriggeredOnRefresh() {
    let em = set_up_test().await;
    let entity = new_test_entity();
    set(&entity, "value", s("hello"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    let id = get(&entity, "id");

    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false); // Reset the tracking of the postLoad invocation

    let reference = em
        .find(&class("LifecycleCallbackTestEntity"), id)
        .await
        .unwrap()
        .unwrap();
    assert!(ENTITY_POST_LOAD_INVOKED.load());
    ENTITY_POST_LOAD_INVOKED.store(false);

    em.refresh(&reference).await.unwrap();
    assert!(
        ENTITY_POST_LOAD_INVOKED.load(),
        "postLoad should be invoked when refresh() is called."
    );
}

#[tokio::test]
async fn testCascadedEntitiesCallsPrePersist() {
    let em = set_up_test().await;
    let e1 = new_test_entity();
    let e2 = new_test_entity();

    let c = new_object(&class("LifecycleCallbackCascader"));
    set(
        &c,
        "entities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
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
async fn testLifecycleCallbacksGetInherited() {
    let em = set_up_test().await;
    let child_meta = em
        .get_class_metadata(&class("LifecycleCallbackChildEntity"))
        .unwrap();
    let callbacks: Vec<(String, Vec<String>)> = child_meta
        .read()
        .lifecycle_callbacks
        .iter()
        .map(|(k, v)| (k.to_owned(), v.clone()))
        .collect();
    assert_eq!(
        callbacks,
        vec![("prePersist".to_owned(), vec!["doStuff".to_owned()])]
    );
}

#[tokio::test]
async fn testLifecycleCallbackEventArgs() {
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

const TEST_ENTITY_DQL: &str = "Doctrine\\Tests\\ORM\\Functional\\LifecycleCallbackTestEntity";

/// `new LifecycleCallbackCascader()`.
fn new_cascader() -> Object {
    let c = new_object(&class("LifecycleCallbackCascader"));
    set(
        &c,
        "entities",
        ArrayCollectionObject::value(PhpArray::new()),
    );
    c
}

/// A persisted cascader of two persisted test entities, after `clear()`.
async fn cascaded_entities(
    em: &Arc<php2rust_doctrine_orm::entity_manager::EntityManager>,
) -> (Object, Object, Object) {
    let e1 = new_test_entity();
    let e2 = new_test_entity();

    let c = new_cascader();
    em.persist(&c).unwrap();

    append(&c, "entities", Value::Object(e1.clone()));
    append(&c, "entities", Value::Object(e2.clone()));
    set_obj(&e1, "cascader", &c);
    set_obj(&e2, "cascader", &c);

    em.flush().await.unwrap();
    em.clear().unwrap();
    (c, e1, e2)
}

#[tokio::test]
async fn testPreSavePostSaveCallbacksAreInvoked() {
    let em = set_up_test().await;
    let entity = new_test_entity();
    set(&entity, "value", s("hello"));
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();

    assert_eq!(get(&entity, "prePersistCallbackInvoked"), Value::Bool(true));
    assert_eq!(
        get(&entity, "postPersistCallbackInvoked"),
        Value::Bool(true)
    );

    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false); // Reset the tracking of the postLoad invocation

    let query = em.create_query(&format!("select e from {TEST_ENTITY_DQL} e"));
    let result = list(&query.get_result(HYDRATE_OBJECT).await.unwrap());
    assert!(ENTITY_POST_LOAD_INVOKED.load());

    set(&obj(&result[0]), "value", s("hello again"));

    em.flush().await.unwrap();

    assert_eq!(
        s("changed from preUpdate callback!"),
        get(&obj(&result[0]), "value")
    );
}

#[tokio::test]
async fn testPostLoadIsInvokedOnFetchJoinedEntities() {
    let em = set_up_test().await;
    let ent_a = new_cascader();
    em.persist(&ent_a).unwrap();

    let ent_b1 = new_test_entity();
    let ent_b2 = new_test_entity();

    append(&ent_a, "entities", Value::Object(ent_b1.clone()));
    append(&ent_a, "entities", Value::Object(ent_b2.clone()));
    set_obj(&ent_b1, "cascader", &ent_a);
    set_obj(&ent_b2, "cascader", &ent_a);

    em.flush().await.unwrap();
    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false); // Reset the tracking of the postLoad invocation
    CASCADER_POST_LOAD_INVOKED.store(false);

    let dql = "SELECT
    entA, entB
FROM
    Doctrine\\Tests\\ORM\\Functional\\LifecycleCallbackCascader AS entA
LEFT JOIN
    entA.entities AS entB
WHERE
    entA.id = :entA_id";

    let query = em.create_query(dql);
    query.set_parameter("entA_id", get(&ent_a, "id"), None);
    let fetched_a = obj(&query.get_one_or_null_result(None).await.unwrap());

    assert!(CASCADER_POST_LOAD_INVOKED.load());
    for _fetch_joined_ent_b in elements(&fetched_a, "entities").await {
        assert!(ENTITY_POST_LOAD_INVOKED.load());
    }
}

#[tokio::test]
async fn testCascadedEntitiesNotLoadedInPostLoadDuringIteration() {
    let em = set_up_test().await;
    let (_c, e1, e2) = cascaded_entities(&em).await;
    ENTITY_POST_LOAD_INVOKED.store(false); // Reset the tracking of the postLoad invocation
    CASCADER_POST_LOAD_INVOKED.store(false);

    let dql = format!(
        "SELECT
    e, c
FROM
    {TEST_ENTITY_DQL} AS e
LEFT JOIN
    e.cascader AS c
WHERE
    e.id IN ({}, {})",
        to_text(&get(&e1, "id")),
        to_text(&get(&e2, "id"))
    );

    let query = em.create_query(&dql);

    let mut iterable_result = query.to_iterable(None, HYDRATE_OBJECT).await.unwrap();

    if let Some(entity) = iterable_result.next().await {
        let entity = &entity.unwrap();
        assert!(ENTITY_POST_LOAD_INVOKED.load());
        assert_eq!(
            Value::Bool(false),
            get(&obj(entity), "postLoadCascaderNotNull")
        );
    }
}

#[tokio::test]
async fn testCascadedEntitiesNotLoadedInPostLoadDuringIterationWithSimpleObjectHydrator() {
    let em = set_up_test().await;
    em.persist(&new_test_entity()).unwrap();
    em.persist(&new_test_entity()).unwrap();

    em.flush().await.unwrap();
    em.clear().unwrap();
    ENTITY_POST_LOAD_INVOKED.store(false); // Reset the tracking of the postLoad invocation

    let query = em.create_query(&format!("SELECT e FROM {TEST_ENTITY_DQL} AS e"));
    let mut result = query.to_iterable(None, HYDRATE_SIMPLEOBJECT).await.unwrap();

    if let Some(entity) = result.next().await {
        let entity = &entity.unwrap();
        assert!(ENTITY_POST_LOAD_INVOKED.load());
        assert_eq!(
            Value::Bool(false),
            get(&obj(entity), "postLoadCascaderNotNull")
        );
    }
}

/// `LifecycleListenerPreUpdate`.
struct LifecycleListenerPreUpdate;

impl EventListener for LifecycleListenerPreUpdate {
    fn call(&self, _event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        let event_args = (event_args as &dyn Any)
            .downcast_ref::<PreUpdateEventArgs>()
            .expect("PreUpdateEventArgs");
        event_args.set_new_value("name", s("Bob"))?;
        Ok(())
    }
}

#[tokio::test]
async fn testLifecycleListenerChangeUpdateChangeSet() {
    let em = set_up_test().await;
    let listener: Arc<dyn EventListener> = Arc::new(LifecycleListenerPreUpdate);
    em.with_event_manager(|event_manager| {
        event_manager.add_event_listener(&[events::PRE_UPDATE], listener.clone());
    });

    let user = new_object(&class("LifecycleCallbackTestUser"));
    set(&user, "name", s("Bob"));
    set(&user, "value", s("value"));
    em.persist(&user).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    let dql = "SELECT u FROM Doctrine\\Tests\\ORM\\Functional\\LifecycleCallbackTestUser u WHERE u.name = 'Bob'";
    let bob = obj(&em.create_query(dql).get_single_result(None).await.unwrap());
    set(&bob, "name", s("Alice"));

    em.flush().await.unwrap(); // preUpdate reverts Alice to Bob
    em.clear().unwrap();

    em.with_event_manager(|event_manager| {
        event_manager.remove_event_listener(&[events::PRE_UPDATE], &listener);
    });

    let bob = obj(&em.create_query(dql).get_single_result(None).await.unwrap());

    assert_eq!(s("Bob"), get(&bob, "name"));
}
