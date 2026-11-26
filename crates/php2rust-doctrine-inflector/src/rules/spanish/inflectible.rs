//! Upstream: doctrine/inflector src/Rules/Spanish/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\Spanish\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/ereses$/")?, "erés"),
            Transformation::new(Pattern::new("/iones$/")?, "ión"),
            Transformation::new(Pattern::new("/ces$/")?, "z"),
            Transformation::new(Pattern::new("/es$/")?, ""),
            Transformation::new(Pattern::new("/s$/")?, ""),
        ])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/ú([sn])$/i")?, r"u\1es"),
            Transformation::new(Pattern::new("/ó([sn])$/i")?, r"o\1es"),
            Transformation::new(Pattern::new("/í([sn])$/i")?, r"i\1es"),
            Transformation::new(Pattern::new("/é([sn])$/i")?, r"e\1es"),
            Transformation::new(Pattern::new("/á([sn])$/i")?, r"a\1es"),
            Transformation::new(Pattern::new("/z$/i")?, "ces"),
            Transformation::new(Pattern::new("/([aeiou]s)$/i")?, r"\1"),
            Transformation::new(Pattern::new("/([^aeéiou])$/i")?, r"\1es"),
            Transformation::new(Pattern::new("/$/")?, "s"),
        ])
    }

    pub fn get_irregular() -> Vec<Substitution> {
        vec![
            Substitution::new(Word::new("el"), Word::new("los")),
            Substitution::new(Word::new("papá"), Word::new("papás")),
            Substitution::new(Word::new("mamá"), Word::new("mamás")),
            Substitution::new(Word::new("sofá"), Word::new("sofás")),
            Substitution::new(Word::new("mes"), Word::new("meses")),
        ]
    }
}
