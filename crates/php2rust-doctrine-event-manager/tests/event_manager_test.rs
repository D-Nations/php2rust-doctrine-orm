//! EventManagerTest.php at doctrine/event-manager
//! dda33921b198841ca8dbad2eaa5d4d34769d18cf. Names and assertions are
//! PHPUnit's. The test case is its own listener (`$this`); here it is the
//! `TestCase` struct, and `setUp()` is `TestCase::set_up()`.
//!
//! Upstream: doctrine/event-manager tests/EventManagerTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use php2rust_doctrine_event_manager::{
    EmptyEventArgs, EventArgs, EventDispatcher, EventListener, EventListenerIntrospector,
    EventListenerRegistry, EventManager, EventSubscriber, EventSubscriberRegistry, ListenerError,
    undefined_method,
};

/* Some pseudo events */
const PRE_FOO: &str = "preFoo";
const POST_FOO: &str = "postFoo";
const PRE_BAR: &str = "preBar";

/// The listener half of `EventManagerTest`.
#[derive(Default)]
struct Listener {
    pre_foo_invoked: AtomicBool,
    post_foo_invoked: AtomicBool,
}

impl EventListener for Listener {
    /* Listener methods */
    fn call(&self, event_name: &str, _e: &dyn EventArgs) -> Result<(), ListenerError> {
        match event_name {
            "preFoo" => self.pre_foo_invoked.store(true, Ordering::SeqCst),
            "postFoo" => self.post_foo_invoked.store(true, Ordering::SeqCst),
            _ => {
                return Err(undefined_method(
                    "Doctrine\\Tests\\Common\\EventManagerTest",
                    event_name,
                ));
            }
        }
        Ok(())
    }
}

struct TestCase {
    this: Arc<Listener>,
    event_manager: EventManager,
}

impl TestCase {
    fn set_up() -> Self {
        Self {
            this: Arc::new(Listener::default()),
            event_manager: EventManager::new(),
        }
    }

    fn this(&self) -> Arc<dyn EventListener> {
        self.this.clone()
    }
}

#[test]
fn testInitialState() {
    let t = TestCase::set_up();
    assert!(t.event_manager.get_all_listeners().is_empty());
    assert!(!t.event_manager.has_listeners(PRE_FOO));
    assert!(!t.event_manager.has_listeners(POST_FOO));
}

#[test]
fn testAddEventListener() {
    let mut t = TestCase::set_up();
    let this = t.this();
    t.event_manager
        .add_event_listener(&["preFoo", "postFoo"], this);
    assert!(t.event_manager.has_listeners(PRE_FOO));
    assert!(t.event_manager.has_listeners(POST_FOO));
    assert_eq!(1, t.event_manager.get_listeners(PRE_FOO).len());
    assert_eq!(1, t.event_manager.get_listeners(POST_FOO).len());
    assert_eq!(2, t.event_manager.get_all_listeners().len());
    assert_eq!(
        vec!["preFoo", "postFoo"],
        t.event_manager
            .get_all_listeners()
            .iter()
            .map(|(event, _)| event.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn testDispatchEvent() {
    let mut t = TestCase::set_up();
    let this = t.this();
    t.event_manager
        .add_event_listener(&["preFoo", "postFoo"], this);
    t.event_manager.dispatch_event(PRE_FOO, None).unwrap();
    assert!(t.this.pre_foo_invoked.load(Ordering::SeqCst));
    assert!(!t.this.post_foo_invoked.load(Ordering::SeqCst));
}

#[test]
fn testRemoveEventListener() {
    let mut t = TestCase::set_up();
    let this = t.this();
    t.event_manager
        .add_event_listener(&["preBar"], this.clone());
    assert!(t.event_manager.has_listeners(PRE_BAR));
    t.event_manager.remove_event_listener(&["preBar"], &this);
    assert!(!t.event_manager.has_listeners(PRE_BAR));
}

#[test]
fn testAddEventSubscriber() {
    let mut t = TestCase::set_up();
    let event_subscriber: Arc<dyn EventSubscriber> = Arc::new(TestEventSubscriber);
    t.event_manager.add_event_subscriber(event_subscriber);
    assert!(t.event_manager.has_listeners(PRE_FOO));
    assert!(t.event_manager.has_listeners(POST_FOO));
}

#[test]
fn testRemoveEventSubscriber() {
    let mut t = TestCase::set_up();
    let event_subscriber: Arc<dyn EventSubscriber> = Arc::new(TestEventSubscriber);
    t.event_manager
        .add_event_subscriber(event_subscriber.clone());
    t.event_manager.remove_event_subscriber(&event_subscriber);
    assert!(!t.event_manager.has_listeners(PRE_FOO));
    assert!(!t.event_manager.has_listeners(POST_FOO));
}

#[test]
fn testNoDispatchingForUnregisteredEvent() {
    let t = TestCase::set_up();
    // ReflectionProperty(EventArgs::class, 'emptyEventArgsInstance')
    EmptyEventArgs::set_empty_instance_property(None);
    t.event_manager.dispatch_event("unknown", None).unwrap();
    assert!(EmptyEventArgs::empty_instance_property().is_none());
}

#[test]
fn testEmptyListenersListForUnregisteredEvent() {
    let t = TestCase::set_up();
    assert!(t.event_manager.get_listeners("unknown").is_empty());
}

struct TestEventSubscriber;

impl EventListener for TestEventSubscriber {
    fn call(&self, event_name: &str, _e: &dyn EventArgs) -> Result<(), ListenerError> {
        Err(undefined_method(
            "Doctrine\\Tests\\Common\\TestEventSubscriber",
            event_name,
        ))
    }
}

impl EventSubscriber for TestEventSubscriber {
    fn get_subscribed_events(&self) -> Vec<String> {
        vec!["preFoo".into(), "postFoo".into()]
    }
}
