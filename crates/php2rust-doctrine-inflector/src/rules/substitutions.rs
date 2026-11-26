//! Upstream: doctrine/inflector src/Rules/Substitutions.php

use php_value::PhpString;

use super::substitution::Substitution;
use crate::php::strtolower;
use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\Rules\Substitutions`: keyed by the from word; a
/// later substitution for the same word replaces the earlier one in its
/// place (PHP's array).
#[derive(Debug, Clone, Default)]
pub struct Substitutions {
    substitutions: Vec<(PhpString, Substitution)>,
}

impl Substitutions {
    pub fn new(substitutions: impl IntoIterator<Item = Substitution>) -> Self {
        let mut keyed: Vec<(PhpString, Substitution)> = Vec::new();
        for substitution in substitutions {
            let key = substitution.get_from().get_word().clone();
            match keyed.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = substitution,
                None => keyed.push((key, substitution)),
            }
        }
        Self {
            substitutions: keyed,
        }
    }

    #[must_use]
    pub fn get_flipped_substitutions(&self) -> Substitutions {
        Substitutions::new(self.substitutions.iter().map(|(_, substitution)| {
            Substitution::new(
                substitution.get_to().clone(),
                substitution.get_from().clone(),
            )
        }))
    }
}

impl WordInflector for Substitutions {
    fn inflect(&self, word: &[u8]) -> PhpString {
        let lower_word = strtolower(word);

        if let Some((_, substitution)) = self
            .substitutions
            .iter()
            .find(|(key, _)| key.as_bytes() == lower_word.as_slice())
        {
            let first_letter_uppercase = lower_word.first() != word.first();

            let to_word = substitution.get_to().get_word().as_bytes();

            if first_letter_uppercase {
                let mut out = to_word.to_vec();
                if let Some(first) = out.first_mut() {
                    first.make_ascii_uppercase();
                }
                return PhpString::from(out);
            }

            return PhpString::from(to_word);
        }

        PhpString::from(word)
    }
}
