//! Port of doctrine/persistence 4.2.1: the interfaces the Doctrine object
//! mappers share (`ObjectManager`, `ObjectRepository`, `ManagerRegistry`,
//! `ClassMetadata`, `MappingDriver`, `Proxy`, ...), the
//! `AbstractManagerRegistry` and `AbstractClassMetadataFactory` base
//! classes, the mapping drivers and file locators, the event arguments and
//! the reflection properties.
//!
//! PHP interfaces are traits over shared objects (`Arc<dyn Trait>`, with
//! `&self` methods); abstract classes are traits whose provided methods
//! are the concrete ones, over a state struct. Objects are
//! `php_value::Object`s and what PHP throws is a `php_value::Thrown`.
//! The classes PHP's reflection finds are declared in [`php_reflection`].
//! See README.md for the contract.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! ```
//! use php2rust_doctrine_persistence::mapping::driver::{MappingDriver, MappingDriverChain};
//! use php2rust_doctrine_persistence::mapping::RuntimeReflectionService;
//! use php2rust_doctrine_persistence::mapping::ReflectionService;
//! use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};
//!
//! register_class(ClassInfo::class("App\\Entity\\Base"));
//! register_class(ClassInfo::class("App\\Entity\\User").extends("App\\Entity\\Base"));
//!
//! let reflection = RuntimeReflectionService::new();
//! assert_eq!(
//!     reflection.get_parent_classes("App\\Entity\\User").unwrap(),
//!     ["App\\Entity\\Base"]
//! );
//! assert_eq!(reflection.get_class_short_name("App\\Entity\\User").unwrap(), "User");
//!
//! let chain = MappingDriverChain::new();
//! assert!(chain.is_transient("App\\Entity\\User").unwrap());
//! let error = reflection.get_parent_classes("App\\Missing").unwrap_err();
//! assert_eq!(error.message, "Class 'App\\Missing' does not exist");
//! ```
//!
//! Upstream: none

#![forbid(unsafe_code)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

mod abstract_manager_registry;
pub mod cache;
mod connection_registry;
pub mod event;
mod manager_registry;
pub mod mapping;
mod notify_property_changed;
mod object_manager;
mod object_manager_decorator;
mod object_repository;
pub mod php_reflection;
mod property_changed_listener;
pub mod proxy;
pub mod reflection;

pub use abstract_manager_registry::{AbstractManagerRegistry, ManagerRegistryState, Service};
pub use connection_registry::{Connection, ConnectionRegistry};
pub use manager_registry::{ManagerRegistry, Managers};
pub use notify_property_changed::NotifyPropertyChanged;
pub use object_manager::ObjectManager;
pub use object_manager_decorator::ObjectManagerDecorator;
pub use object_repository::ObjectRepository;
pub use property_changed_listener::PropertyChangedListener;
pub use proxy::Proxy;
