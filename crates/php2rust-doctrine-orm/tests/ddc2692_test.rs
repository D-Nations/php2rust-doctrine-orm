//! Ticket/DDC2692Test.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa,
//! on `SQLite` (its own models).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/Ticket/DDC2692Test.php

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
use std::sync::atomic::{AtomicI64, Ordering};

use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventManager, EventSubscriber, EventSubscriberRegistry, ListenerError,
};
use php2rust_doctrine_orm::events;

use orm::*;
use ticket::*;

/// The anonymous `EventSubscriber` of `preFlush`.
#[derive(Default)]
struct Listener {
    registered_calls: AtomicI64,
}

impl EventListener for Listener {
    fn call(&self, event_name: &str, _: &dyn EventArgs) -> Result<(), ListenerError> {
        assert_eq!(events::PRE_FLUSH, event_name);
        self.registered_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl EventSubscriber for Listener {
    fn get_subscribed_events(&self) -> Vec<String> {
        vec![events::PRE_FLUSH.to_owned()]
    }
}

#[tokio::test]
async fn testIsListenerCalledOnlyOnceOnPreFlush() {
    let listener = Arc::new(Listener::default());
    let mut event_manager = EventManager::new();
    event_manager.add_event_subscriber(listener.clone());
    let em = entity_manager(connection(), Some(event_manager));
    create_schema(&em, &[class("DDC2692Foo").as_str()]).await;
    em.clear().unwrap();

    em.persist(&new("DDC2692Foo")).unwrap();
    em.persist(&new("DDC2692Foo")).unwrap();
    em.flush().await.unwrap();
    em.clear().unwrap();

    assert_eq!(1, listener.registered_calls.load(Ordering::SeqCst));
}
