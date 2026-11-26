//! Upstream: doctrine/inflector src/Rules/Esperanto/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\Esperanto\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![Transformation::new(Pattern::new("oj$")?, "o")])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![Transformation::new(Pattern::new("o$")?, "oj")])
    }

    pub fn get_irregular() -> Vec<Substitution> {
        vec![Substitution::new(Word::new(""), Word::new(""))]
    }
}
