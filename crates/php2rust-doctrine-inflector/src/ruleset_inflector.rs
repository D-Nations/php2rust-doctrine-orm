//! Inflects based on multiple rulesets.
//!
//! Rules:
//! - If the word matches any uninflected word pattern, it is not inflected
//! - The first ruleset that returns a different value for an irregular word wins
//! - The first ruleset that returns a different value for a regular word wins
//! - If none of the above match, the word is left as-is
//!
//! Upstream: doctrine/inflector src/RulesetInflector.php

use php_value::PhpString;

use crate::rules::Ruleset;
use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\RulesetInflector`.
#[derive(Clone)]
pub struct RulesetInflector {
    rulesets: Vec<Ruleset>,
}

impl RulesetInflector {
    pub fn new(ruleset: Ruleset, rulesets: impl IntoIterator<Item = Ruleset>) -> Self {
        let mut all = vec![ruleset];
        all.extend(rulesets);
        Self { rulesets: all }
    }
}

impl WordInflector for RulesetInflector {
    fn inflect(&self, word: &[u8]) -> PhpString {
        if word.is_empty() {
            return PhpString::default();
        }

        for ruleset in &self.rulesets {
            if ruleset.get_uninflected().matches(word) {
                return PhpString::from(word);
            }

            let inflected = ruleset.get_irregular().inflect(word);

            if inflected.as_bytes() != word {
                return inflected;
            }

            let inflected = ruleset.get_regular().inflect(word);

            if inflected.as_bytes() != word {
                return inflected;
            }
        }

        PhpString::from(word)
    }
}
