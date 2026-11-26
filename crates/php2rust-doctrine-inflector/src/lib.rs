//! Port of doctrine/inflector 2.1.0: singular and plural forms in eight
//! languages, and table, class, camel-case and URL forms of words.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

mod cached_word_inflector;
mod error;
mod inflector;
mod noop_word_inflector;
mod php;
pub mod rules;
mod ruleset_inflector;
mod word_inflector;

pub use cached_word_inflector::CachedWordInflector;
pub use error::InflectorError;
pub use inflector::{CAPITALIZE_DELIMITERS, Inflector};
pub use noop_word_inflector::NoopWordInflector;
pub use ruleset_inflector::RulesetInflector;
pub use word_inflector::WordInflector;
