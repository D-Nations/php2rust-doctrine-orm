//! `Doctrine\Inflector\Rules`: the rules a ruleset inflector applies, and
//! each language's rules.
//!
//! Upstream: none

pub mod english;
pub mod esperanto;
pub mod french;
pub mod italian;
pub mod norwegian_bokmal;
mod pattern;
mod patterns;
pub mod portuguese;
mod ruleset;
pub mod spanish;
mod substitution;
mod substitutions;
mod transformation;
mod transformations;
pub mod turkish;
mod word;

pub use pattern::Pattern;
pub use patterns::{Patterns, WordMatcher};
pub use ruleset::Ruleset;
pub use substitution::Substitution;
pub use substitutions::Substitutions;
pub use transformation::Transformation;
pub use transformations::Transformations;
pub use word::Word;
