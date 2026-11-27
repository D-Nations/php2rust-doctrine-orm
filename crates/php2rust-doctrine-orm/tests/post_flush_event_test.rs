//! Port of `PostFlushEventTest`, on `SQLite` with the `cms` model set. The
//! listener is added to the event manager the entity manager is created
//! with.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/PostFlushEventTest.php

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

use std::sync::{Arc, Mutex};

use php_value::Object;
use php2rust_doctrine_event_manager::{
    EventArgs, EventListener, EventListenerRegistry, EventManager, ListenerError, undefined_method,
};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::event::ManagerEventArgs;
use php2rust_doctrine_orm::events;

use orm::{CMS_MODELS, CMS_USER, create_schema, new_object, s, set};

/// `PostFlushListener`: `wasNotified` and `receivedArgs` (the entity
/// manager the arguments give).
#[derive(Default)]
struct PostFlushListener {
    received_em: Mutex<Option<Arc<EntityManager>>>,
}

impl PostFlushListener {
    fn was_notified(&self) -> bool {
        self.received_em.lock().unwrap().is_some()
    }
}

impl EventListener for PostFlushListener {
    fn call(&self, event_name: &str, args: &dyn EventArgs) -> Result<(), ListenerError> {
        if event_name != events::POST_FLUSH {
            return Err(undefined_method(
                "Doctrine\\Tests\\ORM\\Functional\\PostFlushListener",
                event_name,
            ));
        }
        let args: &dyn std::any::Any = args;
        let args = args.downcast_ref::<ManagerEventArgs>().unwrap();
        *self.received_em.lock().unwrap() = Some(Arc::clone(args.get_object_manager()));
        Ok(())
    }
}

/// `setUp()`.
async fn set_up() -> (Arc<EntityManager>, Arc<PostFlushListener>) {
    let listener = Arc::new(PostFlushListener::default());
    let mut evm = EventManager::new();
    evm.add_event_listener(&[events::POST_FLUSH], listener.clone());
    let em = orm::entity_manager(orm::connection(), Some(evm));
    create_schema(&em, &CMS_MODELS).await;
    (em, listener)
}

/// `createNewValidUser()`.
fn create_new_valid_user() -> Object {
    let user = new_object(CMS_USER);
    set(&user, "username", s("dfreudenberger"));
    set(&user, "name", s("Daniel Freudenberger"));
    user
}

#[tokio::test]
async fn testListenerShouldBeNotified() {
    let (em, listener) = set_up().await;
    em.persist(&create_new_valid_user()).unwrap();
    em.flush().await.unwrap();
    assert!(listener.was_notified());
}

#[tokio::test]
async fn testListenerShouldNotBeNotifiedWhenFlushThrowsException() {
    let (em, listener) = set_up().await;
    let user = new_object(CMS_USER);
    set(&user, "username", s("dfreudenberger"));
    em.persist(&user).unwrap();

    let exception_raised = em.flush().await.is_err();

    assert!(exception_raised);
    assert!(!listener.was_notified());
}

#[tokio::test]
async fn testListenerShouldReceiveEntityManagerThroughArgs() {
    let (em, listener) = set_up().await;
    em.persist(&create_new_valid_user()).unwrap();
    em.flush().await.unwrap();
    let received_em = listener.received_em.lock().unwrap().clone().unwrap();
    assert!(Arc::ptr_eq(&em, &received_em));
}
