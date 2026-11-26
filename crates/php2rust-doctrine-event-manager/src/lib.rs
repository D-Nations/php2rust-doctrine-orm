//! Port of doctrine/event-manager 2.1.1: listeners registered on a manager
//! and events dispatched through it.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

mod event_args;

pub use event_args::{EmptyEventArgs, EventArgs};
