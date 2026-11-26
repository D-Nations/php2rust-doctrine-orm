//! Ticket/DDC3123Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (the cms models).
//! The listener reads the scheduled extra updates through
//! `get_scheduled_extra_updates()`, where PHP reads the private
//! `$extraUpdates` through `ReflectionProperty`.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC3123Test.php

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
use std::sync::atomic::{AtomicUsize, Ordering};

use php_value::{Thrown, php_array};
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError,
};
use php2rust_doctrine_orm::event::ManagerEventArgs;
use php2rust_doctrine_orm::events;

use orm::*;

/// The anonymous `postFlush` listener.
#[derive(Default)]
struct PostFlushListener {
    calls: AtomicUsize,
}

impl EventListener for PostFlushListener {
    fn call(&self, event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::POST_FLUSH, event_name);
        self.calls.fetch_add(1, Ordering::SeqCst);
        let args = (event_args as &dyn Any)
            .downcast_ref::<ManagerEventArgs>()
            .expect("PostFlushEventArgs");
        let uow = args.get_object_manager().get_unit_of_work();
        // Assert::assertEmpty($property->getValue($this->uow), ...)
        if !uow.get_scheduled_extra_updates().is_empty() {
            return Err(Box::new(Thrown::new(
                "PHPUnit\\Framework\\ExpectationFailedException",
                "ExtraUpdates are reset before postFlush",
                0,
            )));
        }
        Ok(())
    }
}

#[tokio::test]
async fn testIssue() {
    let listener = Arc::new(PostFlushListener::default());
    let mut event_manager = EventManager::new();
    event_manager.add_event_listener(&[events::POST_FLUSH], listener.clone());
    let em = entity_manager(connection(), Some(event_manager));
    create_schema(&em, &CMS_MODELS).await;

    let user = cms_user();
    let uow = em.get_unit_of_work();

    set(&user, "name", s("Marco"));
    set(&user, "username", s("ocramius"));

    em.persist(&user).unwrap();
    uow.schedule_extra_update(&user, php_array! { "name" => s("changed name") });

    em.flush().await.unwrap();

    assert_eq!(1, listener.calls.load(Ordering::SeqCst));
}
