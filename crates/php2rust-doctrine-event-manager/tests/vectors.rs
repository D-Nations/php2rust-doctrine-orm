//! `EventManager`'s ordering and removal rules as PHP 8.4.27 shows them
//! (`tests/data/event_manager.json`, `oracle/recorders/event_manager.php`):
//! a listener keeps its place when added again and goes last when removed
//! and added; an event keeps its place without listeners, and dispatching
//! it still takes the empty `EventArgs`; a missing method is PHP's `Error`.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::{Arc, Mutex};

use php_value::Thrown;
use php2rust_doctrine_event_manager::{
    EmptyEventArgs, EventArgs, EventDispatcher, EventListener, EventListenerIntrospector,
    EventListenerRegistry, EventManager, ListenerError, undefined_method,
};
use serde_json::{Value as Json, json};

/// The recorder's `Listener`.
struct Listener {
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
}

impl EventListener for Listener {
    fn call(&self, event_name: &str, _e: &dyn EventArgs) -> Result<(), ListenerError> {
        match event_name {
            "a" | "b" => {
                self.log
                    .lock()
                    .unwrap()
                    .push(format!("{}:{event_name}", self.name));
                Ok(())
            }
            _ => Err(undefined_method("Listener", event_name)),
        }
    }
}

fn take(log: &Arc<Mutex<Vec<String>>>) -> Json {
    json!(std::mem::take(&mut *log.lock().unwrap()))
}

#[test]
fn ordering_and_removal_as_php() {
    let data: Json = serde_json::from_str(include_str!("data/event_manager.json")).unwrap();
    let log = Arc::new(Mutex::new(Vec::new()));
    let listener = |name| -> Arc<dyn EventListener> {
        Arc::new(Listener {
            name,
            log: log.clone(),
        })
    };
    let mut em = EventManager::new();
    let (x, y, z) = (listener("x"), listener("y"), listener("z"));

    em.add_event_listener(&["a"], x.clone());
    em.add_event_listener(&["a", "b"], y.clone());
    em.add_event_listener(&["a"], z.clone());
    em.add_event_listener(&["a"], x.clone());
    em.dispatch_event("a", None).unwrap();
    assert_eq!(data["first_dispatch"], take(&log));

    em.remove_event_listener(&["a"], &x);
    em.add_event_listener(&["a"], x.clone());
    em.dispatch_event("a", None).unwrap();
    assert_eq!(data["after_re_adding_x"], take(&log));

    em.remove_event_listener(&["b", "c"], &y);
    let events: Vec<String> = em
        .get_all_listeners()
        .into_iter()
        .map(|(event, _)| event)
        .collect();
    assert_eq!(data["events_after_removing_y"], json!(events));
    assert_eq!(data["has_listeners_b"], json!(em.has_listeners("b")));
    assert_eq!(
        data["count_listeners_b"],
        json!(em.get_listeners("b").len())
    );

    EmptyEventArgs::set_empty_instance_property(None);
    em.dispatch_event("b", None).unwrap();
    assert_eq!(
        data["dispatch_b_creates_empty_instance"],
        json!(EmptyEventArgs::empty_instance_property().is_some())
    );

    em.add_event_listener(&["missing"], x.clone());
    let error = em.dispatch_event("missing", None).unwrap_err();
    let thrown = error.downcast_ref::<Thrown>().unwrap();
    assert_eq!(
        data["missing_method"],
        json!([thrown.class, thrown.message.to_string_lossy()])
    );
}
