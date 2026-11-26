//! Upstream: doctrine/inflector src/Rules/Turkish/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\Turkish\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![Transformation::new(Pattern::new("/l[ae]r$/i")?, "")])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/([eöiü][^aoıueöiü]{0,6})$/u")?, r"\1ler"),
            Transformation::new(Pattern::new("/([aoıu][^aoıueöiü]{0,6})$/u")?, r"\1lar"),
        ])
    }

    pub fn get_irregular() -> Vec<Substitution> {
        vec![
            Substitution::new(Word::new("ben"), Word::new("biz")),
            Substitution::new(Word::new("sen"), Word::new("siz")),
            Substitution::new(Word::new("o"), Word::new("onlar")),
        ]
    }
}
