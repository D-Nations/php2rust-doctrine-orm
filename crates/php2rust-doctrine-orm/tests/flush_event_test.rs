//! Port of `FlushEventTest`, on `SQLite` with the `cms` model set. The
//! `onFlush` listener of `testPersistNewEntitiesOnPreFlush` computes the
//! change set from its synchronous body with
//! `UnitOfWork::compute_change_set_now()`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/FlushEventTest.php

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

use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use php_value::{Key, Value};

use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError,
};
use php2rust_doctrine_orm::collection_object as coll;
use php2rust_doctrine_orm::event::ManagerEventArgs;
use php2rust_doctrine_orm::events;
use php2rust_doctrine_orm::persistent_collection::PersistentCollection;

use orm::{
    CMS_MODELS, CMS_USER, add_phonenumber, cms_user, create_schema, get, get_object, same, set,
};

/// `OnFlushListener`.
struct OnFlushListener;

impl EventListener for OnFlushListener {
    fn call(&self, event_name: &str, args: &dyn EventArgs) -> Result<(), ListenerError> {
        if event_name != events::ON_FLUSH {
            return Err(php2rust_doctrine_event_manager::undefined_method(
                "Doctrine\\Tests\\ORM\\Functional\\OnFlushListener",
                event_name,
            ));
        }
        let args = (args as &dyn Any)
            .downcast_ref::<ManagerEventArgs>()
            .expect("OnFlushEventArgs");

        let em = args.get_object_manager();
        let uow = em.get_unit_of_work();

        for entity in uow.get_scheduled_entity_insertions() {
            if entity.class() == CMS_USER {
                // Adds a phonenumber to every newly persisted CmsUser ...

                let phone = orm::new_object("Doctrine\\Tests\\Models\\CMS\\CmsPhonenumber");
                set(&phone, "phonenumber", Value::Int(12345));
                // Update object model
                add_phonenumber(&entity, &phone);
                // Invoke regular persist call
                em.persist(&phone)?;
                // Explicitly calculate the changeset since onFlush is raised
                // after changeset calculation!
                uow.compute_change_set_now(&em.get_class_metadata(phone.class())?, &phone)?;

                // Take a snapshot because the UoW wont do this for us, because
                // the UoW did not visit this collection.
                // Alternatively we could provide an ->addVisitedCollection() method
                // on the UoW.
                PersistentCollection::from_value(&get(&entity, "phonenumbers"))
                    .expect("a PersistentCollection")
                    .take_snapshot();
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn testPersistNewEntitiesOnPreFlush() {
    let mut evm = EventManager::new();
    evm.add_event_listener(&[events::ON_FLUSH], Arc::new(OnFlushListener));
    let em = orm::entity_manager(orm::connection(), Some(evm));
    create_schema(&em, &CMS_MODELS).await;

    let user = cms_user();
    set(&user, "username", Value::string("romanb"));
    set(&user, "name", Value::string("Roman"));
    set(&user, "status", Value::string("Dev"));

    em.persist(&user).unwrap();

    assert_eq!(0, coll::count(&get(&user, "phonenumbers")).await.unwrap());

    em.flush().await.unwrap();

    let phonenumbers = get(&user, "phonenumbers");
    assert_eq!(1, coll::count(&phonenumbers).await.unwrap());
    let phone = coll::get(&phonenumbers, Key::Int(0)).await.unwrap();
    let Value::Object(phone) = phone else {
        panic!("{phone:?}");
    };
    assert!(em.contains(&phone).unwrap());
    // $user->phonenumbers->get(0)->getUser()
    assert!(same(&get_object(&phone, "user").unwrap(), &user));

    assert!(
        !PersistentCollection::from_value(&phonenumbers)
            .unwrap()
            .is_dirty()
    );
}

/// `OnFlushCalledListener`.
#[derive(Default)]
#[allow(clippy::struct_field_names, reason = "upstream's property names")]
struct OnFlushCalledListener {
    pre_flush: AtomicI64,
    on_flush: AtomicI64,
    post_flush: AtomicI64,
}

impl EventListener for OnFlushCalledListener {
    fn call(&self, event_name: &str, _args: &dyn EventArgs) -> Result<(), ListenerError> {
        let counter = match event_name {
            events::PRE_FLUSH => &self.pre_flush,
            events::ON_FLUSH => &self.on_flush,
            events::POST_FLUSH => &self.post_flush,
            other => {
                return Err(php2rust_doctrine_event_manager::undefined_method(
                    "Doctrine\\Tests\\ORM\\Functional\\OnFlushCalledListener",
                    other,
                ));
            }
        };
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn testPreAndOnFlushCalledAlways() {
    let listener = Arc::new(OnFlushCalledListener::default());
    let mut evm = EventManager::new();
    evm.add_event_listener(&[events::ON_FLUSH], listener.clone());
    evm.add_event_listener(&[events::PRE_FLUSH], listener.clone());
    evm.add_event_listener(&[events::POST_FLUSH], listener.clone());
    let em = orm::entity_manager(orm::connection(), Some(evm));
    create_schema(&em, &CMS_MODELS).await;

    em.flush().await.unwrap();

    assert_eq!(1, listener.pre_flush.load(Ordering::SeqCst));
    assert_eq!(1, listener.on_flush.load(Ordering::SeqCst));

    em.flush().await.unwrap();

    assert_eq!(2, listener.pre_flush.load(Ordering::SeqCst));
    assert_eq!(2, listener.on_flush.load(Ordering::SeqCst));
}
