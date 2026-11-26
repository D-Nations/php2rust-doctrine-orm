//! Interface for event dispatching.
//!
//! This minimal interface is suitable for type-hinting in code that only needs
//! to dispatch events without configuring listeners.
//!
//! Upstream: doctrine/event-manager src/EventDispatcher.php

use crate::event_args::EventArgs;
use crate::event_listener_registry::ListenerError;

/// `Doctrine\Common\EventDispatcher`.
pub trait EventDispatcher {
    /// Dispatches an event to all registered listeners.
    ///
    /// `event_name` is the name of the event to dispatch. The name of the
    /// event is the name of the method that is invoked on listeners.
    /// `event_args` are the event arguments to pass to the event
    /// handlers/listeners. If not supplied, the single empty `EventArgs`
    /// instance is used. What a listener throws ends the dispatch and is
    /// returned.
    fn dispatch_event(
        &self,
        event_name: &str,
        event_args: Option<&dyn EventArgs>,
    ) -> Result<(), ListenerError>;
}
