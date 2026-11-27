//! Port of `FlushEventTest`, on `SQLite` with the `cms` model set.
//! `testPersistNewEntitiesOnPreFlush` is pending: its `onFlush` listener
//! calls `computeChangeSet()`, which is async, from a listener's
//! synchronous call.
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

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError,
};
use php2rust_doctrine_orm::events;

use orm::{CMS_MODELS, create_schema};

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
