//! Upstream: doctrine/inflector src/Rules/NorwegianBokmal/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\NorwegianBokmal\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/re$/i")?, "r"),
            Transformation::new(Pattern::new("/er$/i")?, ""),
        ])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/e$/i")?, "er"),
            Transformation::new(Pattern::new("/r$/i")?, "re"),
            Transformation::new(Pattern::new("/$/")?, "er"),
        ])
    }

    pub fn get_irregular() -> Vec<Substitution> {
        vec![Substitution::new(Word::new("konto"), Word::new("konti"))]
    }
}
