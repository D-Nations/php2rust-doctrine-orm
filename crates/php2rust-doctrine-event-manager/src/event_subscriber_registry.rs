//! Interface for registering and unregistering event subscribers.
//!
//! Upstream: doctrine/event-manager src/EventSubscriberRegistry.php

use std::sync::Arc;

use crate::event_dispatcher::EventDispatcher;
use crate::event_subscriber::EventSubscriber;

/// `Doctrine\Common\EventSubscriberRegistry`.
pub trait EventSubscriberRegistry: EventDispatcher {
    /// Adds an `EventSubscriber`.
    ///
    /// The subscriber is asked for all the events it is interested in and added
    /// as a listener for these events.
    fn add_event_subscriber(&mut self, subscriber: Arc<dyn EventSubscriber>);

    /// Removes an `EventSubscriber`.
    ///
    /// The subscriber is asked for all the events it is interested in and removed
    /// as a listener for these events.
    fn remove_event_subscriber(&mut self, subscriber: &Arc<dyn EventSubscriber>);
}
