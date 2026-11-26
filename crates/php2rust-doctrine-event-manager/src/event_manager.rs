//! The `EventManager` is the central point of Doctrine's event listener system.
//! Listeners are registered on the manager and events are dispatched through the
//! manager.
//!
//! Upstream: doctrine/event-manager src/EventManager.php

use std::sync::Arc;

use crate::event_args::{EmptyEventArgs, EventArgs};
use crate::event_dispatcher::EventDispatcher;
use crate::event_listener_introspector::EventListenerIntrospector;
use crate::event_listener_registry::{
    EventListener, EventListenerRegistry, ListenerError, ListenerId, Listeners,
};
use crate::event_manager_interface::EventManagerInterface;
use crate::event_subscriber::EventSubscriber;
use crate::event_subscriber_registry::EventSubscriberRegistry;

/// `Doctrine\Common\EventManager`.
#[derive(Default, Clone)]
pub struct EventManager {
    /// Map of registered listeners.
    /// <event> => <listeners>
    ///
    /// In PHP's array order: an event keeps its place once added, also
    /// when its last listener is removed.
    listeners: Vec<(String, Listeners)>,
}

impl EventManager {
    pub fn new() -> Self {
        Self::default()
    }

    fn event(&self, event: &str) -> Option<&Listeners> {
        self.listeners
            .iter()
            .find(|(name, _)| name == event)
            .map(|(_, listeners)| listeners)
    }

    fn event_mut(&mut self, event: &str) -> Option<&mut Listeners> {
        self.listeners
            .iter_mut()
            .find(|(name, _)| name == event)
            .map(|(_, listeners)| listeners)
    }
}

impl std::fmt::Debug for EventManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.listeners.iter().map(|(event, listeners)| {
                (
                    event,
                    listeners.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                )
            }))
            .finish()
    }
}

impl EventDispatcher for EventManager {
    fn dispatch_event(
        &self,
        event_name: &str,
        event_args: Option<&dyn EventArgs>,
    ) -> Result<(), ListenerError> {
        // The listeners as they are now: PHP's foreach walks a copy.
        let Some(listeners) = self.event(event_name).cloned() else {
            return Ok(());
        };

        let empty;
        let event_args = if let Some(event_args) = event_args {
            event_args
        } else {
            empty = EmptyEventArgs::get_empty_instance();
            &*empty
        };

        for (_, listener) in listeners {
            listener.call(event_name, event_args)?;
        }
        Ok(())
    }
}

impl EventListenerIntrospector for EventManager {
    fn get_listeners(&self, event: &str) -> Listeners {
        self.event(event).cloned().unwrap_or_default()
    }

    fn get_all_listeners(&self) -> Vec<(String, Listeners)> {
        self.listeners.clone()
    }

    fn has_listeners(&self, event: &str) -> bool {
        self.event(event)
            .is_some_and(|listeners| !listeners.is_empty())
    }
}

impl EventListenerRegistry for EventManager {
    fn add_event_listener(&mut self, events: &[&str], listener: Arc<dyn EventListener>) {
        // Picks the hash code related to that listener
        let oid = ListenerId::of(&listener);

        for &event in events {
            if self.event(event).is_none() {
                self.listeners.push((event.to_owned(), Vec::new()));
            }
            let Some(listeners) = self.event_mut(event) else {
                continue;
            };
            // Overrides listener if a previous one was associated already
            // Prevents duplicate listeners on same event (same instance only)
            match listeners.iter_mut().find(|(id, _)| *id == oid) {
                Some(entry) => entry.1 = listener.clone(),
                None => listeners.push((oid, listener.clone())),
            }
        }
    }

    fn remove_event_listener(&mut self, events: &[&str], listener: &Arc<dyn EventListener>) {
        // Picks the hash code related to that listener
        let oid = ListenerId::of(listener);

        for &event in events {
            if let Some(listeners) = self.event_mut(event) {
                listeners.retain(|(id, _)| *id != oid);
            }
        }
    }
}

impl EventSubscriberRegistry for EventManager {
    fn add_event_subscriber(&mut self, subscriber: Arc<dyn EventSubscriber>) {
        let events = subscriber.get_subscribed_events();
        let events: Vec<&str> = events.iter().map(String::as_str).collect();
        self.add_event_listener(&events, subscriber);
    }

    fn remove_event_subscriber(&mut self, subscriber: &Arc<dyn EventSubscriber>) {
        let events = subscriber.get_subscribed_events();
        let events: Vec<&str> = events.iter().map(String::as_str).collect();
        let listener: Arc<dyn EventListener> = subscriber.clone();
        self.remove_event_listener(&events, &listener);
    }
}

impl EventManagerInterface for EventManager {}
