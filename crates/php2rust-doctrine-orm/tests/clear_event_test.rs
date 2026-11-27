//! Port of `ClearEventTest`. The listener is added to the event manager
//! the entity manager is created with (`getEventManager()` returns a
//! copy here).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ClearEventTest.php

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
use std::sync::atomic::{AtomicBool, Ordering};

use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError, undefined_method,
};
use php2rust_doctrine_orm::events;

/// `OnClearListener`.
#[derive(Default)]
struct OnClearListener {
    called: AtomicBool,
}

impl EventListener for OnClearListener {
    fn call(&self, event_name: &str, _args: &dyn EventArgs) -> Result<(), ListenerError> {
        if event_name != events::ON_CLEAR {
            return Err(undefined_method(
                "Doctrine\\Tests\\ORM\\Functional\\OnClearListener",
                event_name,
            ));
        }
        self.called.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn testEventIsCalledOnClear() {
    let listener = Arc::new(OnClearListener::default());
    let mut event_manager = EventManager::new();
    event_manager.add_event_listener(&[events::ON_CLEAR], listener.clone());
    let em = orm::entity_manager(orm::connection(), Some(event_manager));

    em.clear().unwrap();

    assert!(listener.called.load(Ordering::SeqCst));
}
