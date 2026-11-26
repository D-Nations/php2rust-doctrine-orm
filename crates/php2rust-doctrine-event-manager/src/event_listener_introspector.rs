//! Interface for introspecting event listeners.
//!
//! Provides methods to query registered listeners without modifying them.
//!
//! Upstream: doctrine/event-manager src/EventListenerIntrospector.php

use crate::event_dispatcher::EventDispatcher;
use crate::event_listener_registry::Listeners;

/// `Doctrine\Common\EventListenerIntrospector`.
pub trait EventListenerIntrospector: EventDispatcher {
    /// Gets the listeners of a specific event.
    fn get_listeners(&self, event: &str) -> Listeners;

    /// Gets all listeners keyed by event name.
    fn get_all_listeners(&self) -> Vec<(String, Listeners)>;

    /// Checks whether an event has any registered listeners.
    fn has_listeners(&self, event: &str) -> bool;
}
