//! Interface for registering and unregistering event listeners, and the
//! listener objects themselves (PHP's `object $listener`, whose method named
//! after the event is called).
//!
//! Upstream: doctrine/event-manager src/EventListenerRegistry.php

use std::sync::Arc;

use php_value::Thrown;

use crate::event_args::EventArgs;
use crate::event_dispatcher::EventDispatcher;

/// What a listener throws.
pub type ListenerError = Box<dyn std::error::Error + Send + Sync>;

/// A listener object.
pub trait EventListener: Send + Sync {
    /// `$listener->$eventName($eventArgs)`. A listener without such a
    /// method returns [`undefined_method()`], PHP's `Error`.
    fn call(&self, event_name: &str, event_args: &dyn EventArgs) -> Result<(), ListenerError>;
}

/// PHP's `Error` for a call to a method the listener's class does not have.
pub fn undefined_method(class: &str, method: &str) -> ListenerError {
    Box::new(Thrown::new(
        "Error",
        format!("Call to undefined method {class}::{method}()"),
        0,
    ))
}

/// `spl_object_id($listener)`: the identity of a listener object, the same
/// for every clone of its `Arc` and unique among the objects alive at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ListenerId(usize);

impl ListenerId {
    /// The id of the object behind `listener`.
    pub fn of<T: ?Sized>(listener: &Arc<T>) -> Self {
        Self(Arc::as_ptr(listener).cast::<()>().addr())
    }
}

/// An event's listeners, keyed by their ids, in the order they were first
/// added.
pub type Listeners = Vec<(ListenerId, Arc<dyn EventListener>)>;

/// `Doctrine\Common\EventListenerRegistry`.
pub trait EventListenerRegistry: EventDispatcher {
    /// Adds an event listener that listens on the specified events.
    fn add_event_listener(&mut self, events: &[&str], listener: Arc<dyn EventListener>);

    /// Removes an event listener from the specified events.
    fn remove_event_listener(&mut self, events: &[&str], listener: &Arc<dyn EventListener>);
}
