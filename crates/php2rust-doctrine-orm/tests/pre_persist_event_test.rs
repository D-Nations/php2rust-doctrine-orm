//! Port of `PrePersistEventTest`, on `SQLite`. The listener is added to the
//! event manager the entity manager is created with; the constructors'
//! `uniqid(self::class, true)` is a counter-based id.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/PrePersistEventTest.php

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

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use php_value::{Object, Value};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError, undefined_method,
};
use php2rust_doctrine_orm::event::LifecycleEventArgs;
use php2rust_doctrine_orm::events;

use orm::{get_object, new_object, s, set};

const UNMAPPED: &str = "Doctrine\\Tests\\ORM\\Functional\\EntityWithUnmappedEntity";
const CASCADE: &str = "Doctrine\\Tests\\ORM\\Functional\\EntityWithCascadeAssociation";

/// `new $class()`: the constructor sets `uniqid(self::class, true)`.
fn entity(class: &str) -> Object {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let entity = new_object(class);
    let id = format!("{class}{}", COUNTER.fetch_add(1, Ordering::Relaxed));
    set(&entity, "id", s(&id));
    entity
}

/// `PrePersistUnmappedPersistListener`.
struct PrePersistUnmappedPersistListener;

impl EventListener for PrePersistUnmappedPersistListener {
    fn call(&self, event_name: &str, args: &dyn EventArgs) -> Result<(), ListenerError> {
        if event_name != events::PRE_PERSIST {
            return Err(undefined_method(
                "Doctrine\\Tests\\ORM\\Functional\\PrePersistUnmappedPersistListener",
                event_name,
            ));
        }
        let args: &dyn std::any::Any = args;
        let args = args.downcast_ref::<LifecycleEventArgs>().unwrap();
        let object = args.get_object();

        if object.class() == UNMAPPED {
            let em = args.get_object_manager();
            let uow = em.get_unit_of_work();

            if let Some(unmapped) = get_object(object, "unmapped")
                && !uow.is_in_identity_map(&unmapped).unwrap()
                && !uow.is_scheduled_for_insert(&unmapped)
            {
                em.persist(&unmapped).unwrap();
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn testCallingPersistInPrePersistHook() {
    let mut event_manager = EventManager::new();
    event_manager.add_event_listener(
        &[events::PRE_PERSIST],
        Arc::new(PrePersistUnmappedPersistListener),
    );
    let em = orm::entity_manager(orm::connection(), Some(event_manager));
    orm::create_schema(&em, &[UNMAPPED, CASCADE]).await;

    let entity_with_unmapped = entity(UNMAPPED);
    let entity_with_cascade = entity(CASCADE);

    set(
        &entity_with_unmapped,
        "unmapped",
        Value::Object(entity_with_cascade.clone()),
    );
    set(
        &entity_with_cascade,
        "cascaded",
        Value::Object(entity_with_unmapped.clone()),
    );

    em.persist(&entity_with_unmapped).unwrap();

    assert!(
        em.get_unit_of_work()
            .is_scheduled_for_insert(&entity_with_cascade)
    );
    assert!(
        em.get_unit_of_work()
            .is_scheduled_for_insert(&entity_with_unmapped)
    );
}
