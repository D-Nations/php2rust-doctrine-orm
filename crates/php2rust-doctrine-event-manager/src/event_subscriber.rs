//! An `EventSubscriber` knows what events it is interested in.
//! If an `EventSubscriber` is added to an `EventManager`, the manager invokes
//! `getSubscribedEvents()` and registers the subscriber as a listener for all
//! returned events.
//!
//! Upstream: doctrine/event-manager src/EventSubscriber.php

use crate::event_listener_registry::EventListener;

/// `Doctrine\Common\EventSubscriber`.
pub trait EventSubscriber: EventListener {
    /// Returns an array of events this subscriber wants to listen to.
    fn get_subscribed_events(&self) -> Vec<String>;
}
