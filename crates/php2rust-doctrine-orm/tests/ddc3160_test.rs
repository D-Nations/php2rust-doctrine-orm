//! Ticket/DDC3160Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3160Test.php

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

use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError,
};
use php2rust_doctrine_orm::event::ManagerEventArgs;
use php2rust_doctrine_orm::events;

use orm::*;

/// `DDC3160OnFlushListener`.
#[derive(Default)]
struct DDC3160OnFlushListener {
    inserts: AtomicI64,
    updates: AtomicI64,
}

impl EventListener for DDC3160OnFlushListener {
    fn call(&self, event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::ON_FLUSH, event_name);
        let args = (event_args as &dyn Any)
            .downcast_ref::<ManagerEventArgs>()
            .expect("OnFlushEventArgs");
        let em = args.get_object_manager();
        let uow = em.get_unit_of_work();

        for entity in uow.get_scheduled_entity_insertions() {
            self.inserts.fetch_add(1, Ordering::SeqCst);
            if entity.class() == CMS_USER {
                set(&entity, "username", s("romanc"));
                let cm = em.get_class_metadata(entity.class())?;
                uow.recompute_single_entity_change_set(&cm, &entity)?;
            }
        }

        for _entity in uow.get_scheduled_entity_updates() {
            self.updates.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[tokio::test]
async fn testNoUpdateOnInsert() {
    let listener = Arc::new(DDC3160OnFlushListener::default());
    let mut event_manager = EventManager::new();
    event_manager.add_event_listener(&[events::ON_FLUSH], listener.clone());
    let em = entity_manager(connection(), Some(event_manager));
    create_schema(&em, &CMS_MODELS).await;

    let user = cms_user_with("Roman", "romanb", "Dev");

    em.persist(&user).unwrap();
    em.flush().await.unwrap();

    em.refresh(&user).await.unwrap();

    assert_eq!(s("romanc"), get(&user, "username"));
    assert_eq!(1, listener.inserts.load(Ordering::SeqCst));
    assert_eq!(0, listener.updates.load(Ordering::SeqCst));
}
