//! Upstream: doctrine/inflector src/Rules/Esperanto/Rules.php

use php_regex::PcreError;

use super::inflectible::Inflectible;
use super::uninflected::Uninflected;
use crate::rules::{Patterns, Ruleset, Substitutions, Transformations};

/// `Doctrine\Inflector\Rules\Esperanto\Rules`.
pub struct Rules;

impl Rules {
    pub fn get_singular_ruleset() -> Result<Ruleset, PcreError> {
        Ok(Ruleset::new(
            Transformations::new(Inflectible::get_singular()?),
            Patterns::new(Uninflected::get_singular()?)?,
            Substitutions::new(Inflectible::get_irregular()).get_flipped_substitutions(),
        ))
    }

    pub fn get_plural_ruleset() -> Result<Ruleset, PcreError> {
        Ok(Ruleset::new(
            Transformations::new(Inflectible::get_plural()?),
            Patterns::new(Uninflected::get_plural()?)?,
            Substitutions::new(Inflectible::get_irregular()),
        ))
    }
}
