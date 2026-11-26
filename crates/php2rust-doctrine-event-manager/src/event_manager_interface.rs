//! Provided for convenience, but consider using the individual interfaces directly.
//!
//! Upstream: doctrine/event-manager src/EventManagerInterface.php

use crate::event_listener_introspector::EventListenerIntrospector;
use crate::event_listener_registry::EventListenerRegistry;
use crate::event_subscriber_registry::EventSubscriberRegistry;

/// `Doctrine\Common\EventManagerInterface`.
pub trait EventManagerInterface:
    EventListenerIntrospector + EventListenerRegistry + EventSubscriberRegistry
{
}
