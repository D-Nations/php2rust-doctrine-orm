//! Port of doctrine/instantiator 2.1.0: blank instances of a class without
//! running its constructor, on persistence's class registry
//! (`php2rust_doctrine_persistence::php_reflection`). See README.md for
//! the contract.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

pub mod exception;
mod instantiator;
mod instantiator_interface;

pub use instantiator::{Instantiator, declare_classes};
pub use instantiator_interface::InstantiatorInterface;
