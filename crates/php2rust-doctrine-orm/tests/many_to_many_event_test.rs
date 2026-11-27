//! Port of `ManyToManyEventTest`, on `SQLite` with the `cms` model set. The
//! listener is added to the event manager the entity manager is created
//! with.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/ManyToManyEventTest.php

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

use php_value::Object;
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError, undefined_method,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::events;

use orm::{CMS_MODELS, add_group, cms_group, cms_user, create_schema, remove_element, s, set};

/// `PostUpdateListener`.
#[derive(Default)]
struct PostUpdateListener {
    was_notified: AtomicBool,
}

impl EventListener for PostUpdateListener {
    fn call(&self, event_name: &str, _args: &dyn EventArgs) -> Result<(), ListenerError> {
        if event_name != events::POST_UPDATE {
            return Err(undefined_method(
                "Doctrine\\Tests\\ORM\\Functional\\PostUpdateListener",
                event_name,
            ));
        }
        self.was_notified.store(true, Ordering::SeqCst);
        Ok(())
    }
}

/// `setUp()`.
async fn set_up() -> (Arc<EntityManager>, Arc<PostUpdateListener>) {
    let listener = Arc::new(PostUpdateListener::default());
    let mut evm = EventManager::new();
    evm.add_event_listener(&[events::POST_UPDATE], listener.clone());
    let em = orm::entity_manager(orm::connection(), Some(evm));
    create_schema(&em, &CMS_MODELS).await;
    (em, listener)
}

/// `createNewValidUser()`.
fn create_new_valid_user() -> Object {
    let user = cms_user();
    set(&user, "username", s("fran6co"));
    set(&user, "name", s("Francisco Facioni"));
    let group = cms_group("users");
    add_group(&user, &group);
    user
}

#[tokio::test]
async fn testListenerShouldBeNotifiedWhenNewCollectionEntryAdded() {
    let (em, listener) = set_up().await;
    let user = create_new_valid_user();
    let group = cms_group("admins");

    em.persist(&user).unwrap();
    em.persist(&group).unwrap();
    em.flush().await.unwrap();
    assert!(!listener.was_notified.load(Ordering::SeqCst));

    add_group(&user, &group);
    em.flush().await.unwrap();

    assert!(listener.was_notified.load(Ordering::SeqCst));
}

#[tokio::test]
async fn testListenerShouldBeNotifiedWhenCollectionEntryRemoved() {
    let (em, listener) = set_up().await;
    let user = create_new_valid_user();
    let group = cms_group("admins");
    add_group(&user, &group);

    em.persist(&user).unwrap();
    em.persist(&group).unwrap();
    em.flush().await.unwrap();
    assert!(!listener.was_notified.load(Ordering::SeqCst));

    remove_element(&user, "groups", &group).await;
    em.flush().await.unwrap();

    assert!(listener.was_notified.load(Ordering::SeqCst));
}
