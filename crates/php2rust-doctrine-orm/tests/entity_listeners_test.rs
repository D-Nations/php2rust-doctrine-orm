//! Functional/EntityListenersTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the company models). The listener records the entity and
//! the class of the event arguments (`company::register_listeners()`);
//! `assertInstanceOf(LifecycleEventArgs::class, ...)` checks that class,
//! a subclass of `LifecycleEventArgs`. The anonymous listener classes are
//! `EventListener`s.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/EntityListenersTest.php

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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Weak};

use php_value::{Object, PhpArray, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, ListenerError,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::event;
use php2rust_doctrine_orm::events;

use company::{COMPANY_FIX_CONTRACT, company_contract, company_person};
use orm::{get, same, set};

const COMPANY_CONTRACT_LISTENER: &str = "Doctrine\\Tests\\Models\\Company\\CompanyContractListener";

struct Fixture {
    em: Arc<EntityManager>,
    listener: Object,
}

async fn set_up() -> Fixture {
    company::register_listeners();
    let em = orm::set_up(&company::COMPANY_MODELS).await;

    let listener = em
        .get_configuration()
        .get_entity_listener_resolver()
        .resolve(COMPANY_CONTRACT_LISTENER)
        .unwrap();

    Fixture { em, listener }
}

/// `new CompanyFixContract()` with `setFixPrice($price)`.
fn fix_contract(price: i64) -> Object {
    let fix = company_contract(COMPANY_FIX_CONTRACT);
    set(&fix, "fixPrice", Value::Int(price));
    fix
}

impl Fixture {
    /// `$this->listener->$property = []`.
    fn reset(&self, property: &str) {
        set(&self.listener, property, Value::Array(PhpArray::new()));
    }

    /// `assertCount(1, $calls)`, `assertSame($entity, $calls[0][0])`,
    /// `assertInstanceOf(CompanyFixContract::class, $calls[0][0])` and the
    /// class of `$calls[0][1]`.
    fn assert_one_call(&self, property: &str, entity: &Object, args_class: &str) {
        let Value::Array(calls) = get(&self.listener, property) else {
            panic!("no {property}");
        };
        assert_eq!(1, calls.len(), "{calls:?}");
        let Some(Value::Array(call)) = calls.values().next() else {
            panic!("{calls:?}");
        };
        let Some(Value::Object(called)) = call.values().next() else {
            panic!("{call:?}");
        };
        assert!(same(entity, called));
        assert!(called.instance_of(COMPANY_FIX_CONTRACT));
        assert_eq!(
            Some(&Value::string(args_class)),
            call.values().nth(1),
            "{call:?}"
        );
    }
}

#[tokio::test]
async fn testPreFlushListeners() {
    let f = set_up().await;
    let fix = fix_contract(2000);

    f.reset("preFlushCalls");

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("preFlushCalls", &fix, event::PRE_FLUSH_EVENT_ARGS);
}

#[tokio::test]
async fn testPostLoadListeners() {
    let f = set_up().await;
    let fix = fix_contract(2000);

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();
    f.em.clear().unwrap();

    f.reset("postLoadCalls");

    let dql =
        "SELECT f FROM Doctrine\\Tests\\Models\\Company\\CompanyFixContract f WHERE f.id = ?1";
    let q = f.em.create_query(dql);
    q.set_parameter(1, get(&fix, "id"), None);
    let fix = orm::obj(&q.get_single_result(None).await.unwrap());

    f.assert_one_call("postLoadCalls", &fix, event::POST_LOAD_EVENT_ARGS);
}

#[tokio::test]
async fn testPrePersistListeners() {
    let f = set_up().await;
    let fix = fix_contract(2000);

    f.reset("prePersistCalls");

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("prePersistCalls", &fix, event::PRE_PERSIST_EVENT_ARGS);
}

#[tokio::test]
async fn testPostPersistListeners() {
    let f = set_up().await;
    let fix = fix_contract(2000);

    f.reset("postPersistCalls");

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("postPersistCalls", &fix, event::POST_PERSIST_EVENT_ARGS);
}

/// The anonymous `postPersist` listener: every tracked object has an id.
struct PostPersistTracker {
    tracked_objects: Vec<Object>,
    invocation_count: AtomicUsize,
}

impl EventListener for PostPersistTracker {
    fn call(&self, event_name: &str, _event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::POST_PERSIST, event_name);
        for object in &self.tracked_objects {
            assert!(!get(object, "id").is_null());
        }

        self.invocation_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn testPostPersistCalledAfterAllInsertsHaveBeenPerformedAndIdsHaveBeenAssigned() {
    let f = set_up().await;
    let object1 = fix_contract(2000);

    let object2 = company_person("J. Doe");

    f.em.persist(&object1).unwrap();
    f.em.persist(&object2).unwrap();

    let listener = Arc::new(PostPersistTracker {
        tracked_objects: vec![object1, object2],
        invocation_count: AtomicUsize::new(0),
    });

    f.em.with_event_manager(|evm| {
        evm.add_event_listener(&[events::POST_PERSIST], listener.clone());
    });
    f.em.flush().await.unwrap();

    assert_eq!(2, listener.invocation_count.load(Ordering::SeqCst));
}

#[tokio::test]
async fn testPreUpdateListeners() {
    let f = set_up().await;
    let fix = fix_contract(1000);

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.reset("preUpdateCalls");

    set(&fix, "fixPrice", Value::Int(2000));

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("preUpdateCalls", &fix, event::PRE_UPDATE_EVENT_ARGS);
}

#[tokio::test]
async fn testPostUpdateListeners() {
    let f = set_up().await;
    let fix = fix_contract(1000);

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.reset("postUpdateCalls");

    set(&fix, "fixPrice", Value::Int(2000));

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("postUpdateCalls", &fix, event::POST_UPDATE_EVENT_ARGS);
}

#[tokio::test]
async fn testPreRemoveListeners() {
    let f = set_up().await;
    let fix = fix_contract(1000);

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.reset("preRemoveCalls");

    f.em.remove(&fix).await.unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("preRemoveCalls", &fix, event::PRE_REMOVE_EVENT_ARGS);
}

#[tokio::test]
async fn testPostRemoveListeners() {
    let f = set_up().await;
    let fix = fix_contract(1000);

    f.em.persist(&fix).unwrap();
    f.em.flush().await.unwrap();

    f.reset("postRemoveCalls");

    f.em.remove(&fix).await.unwrap();
    f.em.flush().await.unwrap();

    f.assert_one_call("postRemoveCalls", &fix, event::POST_REMOVE_EVENT_ARGS);
}

/// The anonymous `postRemove` listener: no tracked object is in the
/// identity map.
struct PostRemoveTracker {
    em: Weak<EntityManager>,
    tracked_objects: Vec<Object>,
    invocation_count: AtomicUsize,
}

impl EventListener for PostRemoveTracker {
    fn call(&self, event_name: &str, _event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::POST_REMOVE, event_name);
        let em = self.em.upgrade().expect("the entity manager");
        let uow = em.get_unit_of_work();
        for object in &self.tracked_objects {
            assert!(!uow.is_in_identity_map(object)?);
        }

        self.invocation_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn testPostRemoveCalledAfterAllRemovalsHaveBeenPerformed() {
    let f = set_up().await;
    let object1 = fix_contract(2000);

    let object2 = company_person("J. Doe");

    f.em.persist(&object1).unwrap();
    f.em.persist(&object2).unwrap();
    f.em.flush().await.unwrap();

    let listener = Arc::new(PostRemoveTracker {
        em: Arc::downgrade(&f.em),
        tracked_objects: vec![object1.clone(), object2.clone()],
        invocation_count: AtomicUsize::new(0),
    });

    f.em.with_event_manager(|evm| {
        evm.add_event_listener(&[events::POST_REMOVE], listener.clone());
    });
    f.em.remove(&object1).await.unwrap();
    f.em.remove(&object2).await.unwrap();
    f.em.flush().await.unwrap();

    assert_eq!(2, listener.invocation_count.load(Ordering::SeqCst));
}
