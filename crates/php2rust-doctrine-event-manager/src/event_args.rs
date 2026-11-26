//! `EventArgs` is the base class for classes containing event data.
//!
//! The class is the [`EventArgs`] trait here, which the classes with event
//! data implement; an instance of the class itself, which contains no event
//! data, is an [`EmptyEventArgs`]. A listener gets `&dyn EventArgs` and
//! downcasts it (`(args as &dyn Any).downcast_ref::<T>()`).
//!
//! Upstream: doctrine/event-manager src/EventArgs.php

use std::any::Any;
use std::cell::RefCell;
use std::sync::Arc;

/// `Doctrine\Common\EventArgs`: event data passed to listeners.
pub trait EventArgs: Any + Send + Sync {}

/// `new EventArgs()`: the event arguments of events that do not pass state
/// information to an event handler.
#[derive(Debug, Default)]
pub struct EmptyEventArgs;

impl EventArgs for EmptyEventArgs {}

thread_local! {
    /// Single instance of EventArgs (one per thread, as PHP has one per
    /// request).
    static EMPTY_EVENT_ARGS_INSTANCE: RefCell<Option<Arc<EmptyEventArgs>>> =
        const { RefCell::new(None) };
}

impl EmptyEventArgs {
    /// Gets the single, empty and immutable `EventArgs` instance.
    ///
    /// This instance will be used when events are dispatched without any parameter,
    /// like this: `EventManager::dispatchEvent('eventname')`;
    ///
    /// The benefit from this is that only one empty instance is instantiated and shared
    /// (otherwise there would be instances for every dispatched in the abovementioned form).
    pub fn get_empty_instance() -> Arc<EmptyEventArgs> {
        EMPTY_EVENT_ARGS_INSTANCE.with(|instance| {
            instance
                .borrow_mut()
                .get_or_insert_with(|| Arc::new(EmptyEventArgs))
                .clone()
        })
    }

    /// The private static `$emptyEventArgsInstance`, as a test reads it
    /// through `ReflectionProperty::getValue()`.
    #[doc(hidden)]
    pub fn empty_instance_property() -> Option<Arc<EmptyEventArgs>> {
        EMPTY_EVENT_ARGS_INSTANCE.with(|instance| instance.borrow().clone())
    }

    /// Sets the private static `$emptyEventArgsInstance`, as a test does
    /// through `ReflectionProperty::setValue()`.
    #[doc(hidden)]
    pub fn set_empty_instance_property(value: Option<Arc<EmptyEventArgs>>) {
        EMPTY_EVENT_ARGS_INSTANCE.with(|instance| *instance.borrow_mut() = value);
    }
}
