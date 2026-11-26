//! Upstream: doctrine/inflector src/Rules/French/Inflectible.php

use php_regex::PcreError;

use crate::rules::{Pattern, Substitution, Transformation, Word};

/// `Doctrine\Inflector\Rules\French\Inflectible`.
pub struct Inflectible;

impl Inflectible {
    pub fn get_singular() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(
                Pattern::new("/(b|cor|ém|gemm|soupir|trav|vant|vitr)aux$/")?,
                r"\1ail",
            ),
            Transformation::new(Pattern::new("/ails$/")?, "ail"),
            Transformation::new(Pattern::new("/(journ|chev|loc)aux$/")?, r"\1al"),
            Transformation::new(
                Pattern::new("/(bijou|caillou|chou|genou|hibou|joujou|pou|au|eu|eau)x$/")?,
                r"\1",
            ),
            Transformation::new(Pattern::new("/s$/")?, ""),
        ])
    }

    pub fn get_plural() -> Result<Vec<Transformation>, PcreError> {
        Ok(vec![
            Transformation::new(Pattern::new("/(s|x|z)$/")?, r"\1"),
            Transformation::new(
                Pattern::new("/(b|cor|ém|gemm|soupir|trav|vant|vitr)ail$/")?,
                r"\1aux",
            ),
            Transformation::new(Pattern::new("/ail$/")?, "ails"),
            Transformation::new(
                Pattern::new("/(chacal|carnaval|festival|récital)$/")?,
                r"\1s",
            ),
            Transformation::new(Pattern::new("/al$/")?, "aux"),
            Transformation::new(Pattern::new("/(bleu|émeu|landau|pneu|sarrau)$/")?, r"\1s"),
            Transformation::new(
                Pattern::new("/(bijou|caillou|chou|genou|hibou|joujou|lieu|pou|au|eu|eau)$/")?,
                r"\1x",
            ),
            Transformation::new(Pattern::new("/$/")?, "s"),
        ])
    }

    pub fn get_irregular() -> Vec<Substitution> {
        vec![
            Substitution::new(Word::new("monsieur"), Word::new("messieurs")),
            Substitution::new(Word::new("madame"), Word::new("mesdames")),
            Substitution::new(Word::new("mademoiselle"), Word::new("mesdemoiselles")),
        ]
    }
}
