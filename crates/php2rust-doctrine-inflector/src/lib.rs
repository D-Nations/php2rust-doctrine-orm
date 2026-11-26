//! Port of doctrine/inflector 2.1.0: singular and plural forms in eight
//! languages, and table, class, camel-case and URL forms of words. See
//! README.md for the contract.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

mod cached_word_inflector;
mod error;
mod generic_language_inflector_factory;
mod inflector;
mod inflector_factory;
mod language;
mod language_inflector_factory;
mod noop_word_inflector;
mod php;
pub mod rules;
mod ruleset_inflector;
mod word_inflector;

pub use cached_word_inflector::CachedWordInflector;
pub use error::InflectorError;
pub use generic_language_inflector_factory::GenericLanguageInflectorFactory;
pub use inflector::{CAPITALIZE_DELIMITERS, Inflector};
pub use inflector_factory::InflectorFactory;
pub use language::Language;
pub use language_inflector_factory::LanguageInflectorFactory;
pub use noop_word_inflector::NoopWordInflector;
pub use ruleset_inflector::RulesetInflector;
pub use word_inflector::WordInflector;
