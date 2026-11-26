//! Port of doctrine/event-manager 2.1.1: listeners registered on a manager
//! and events dispatched through it. See README.md for the contract.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

mod event_args;
mod event_dispatcher;
mod event_listener_introspector;
mod event_listener_registry;
mod event_manager;
mod event_manager_interface;
mod event_subscriber;
mod event_subscriber_registry;

pub use event_args::{EmptyEventArgs, EventArgs};
pub use event_dispatcher::EventDispatcher;
pub use event_listener_introspector::EventListenerIntrospector;
pub use event_listener_registry::{
    EventListener, EventListenerRegistry, ListenerError, ListenerId, Listeners, undefined_method,
};
pub use event_manager::EventManager;
pub use event_manager_interface::EventManagerInterface;
pub use event_subscriber::EventSubscriber;
pub use event_subscriber_registry::EventSubscriberRegistry;
