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
//! Upstream: none

#![forbid(unsafe_code)]

pub mod mapping;
pub mod php_reflection;
pub mod proxy;
pub mod reflection;

pub use proxy::Proxy;
