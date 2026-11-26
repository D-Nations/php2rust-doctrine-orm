//! `Doctrine\Inflector\Rules`: the rules a ruleset inflector applies.
//!
//! Upstream: none

mod pattern;
mod patterns;
mod ruleset;
mod substitution;
mod substitutions;
mod transformation;
mod transformations;
mod word;

pub use pattern::Pattern;
pub use patterns::{Patterns, WordMatcher};
pub use ruleset::Ruleset;
pub use substitution::Substitution;
pub use substitutions::Substitutions;
pub use transformation::Transformation;
pub use transformations::Transformations;
pub use word::Word;
