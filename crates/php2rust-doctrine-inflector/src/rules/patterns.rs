//! Upstream: doctrine/inflector src/Rules/Patterns.php

use php_regex::{Pcre, PcreError};
use php_value::PhpString;

use super::pattern::{Pattern, compile};

/// What a ruleset asks of its uninflected patterns (`Patterns::matches()`);
/// test doubles implement it too.
pub trait WordMatcher: Send + Sync {
    fn matches(&self, word: &[u8]) -> bool;
}

/// `Doctrine\Inflector\Rules\Patterns`: one regex,
/// `/^(?:p1|p2|...)$/i`, of the patterns as given (`getPattern()`).
#[derive(Debug, Clone)]
pub struct Patterns {
    regex: PhpString,
    compiled: Option<Pcre>,
}

impl Patterns {
    pub fn new(patterns: impl IntoIterator<Item = Pattern>) -> Result<Self, PcreError> {
        let mut regex = b"/^(?:".to_vec();
        for (i, pattern) in patterns.into_iter().enumerate() {
            if i > 0 {
                regex.push(b'|');
            }
            regex.extend_from_slice(pattern.get_pattern().as_bytes());
        }
        regex.extend_from_slice(b")$/i");
        let compiled = compile(&regex)?;

        Ok(Self {
            regex: PhpString::from(regex),
            compiled,
        })
    }
}

impl WordMatcher for Patterns {
    fn matches(&self, word: &[u8]) -> bool {
        self.compiled
            .as_ref()
            .is_some_and(|regex| regex.is_match(word) == Ok(true))
    }
}

impl Patterns {
    /// The combined regex.
    pub fn regex(&self) -> &PhpString {
        &self.regex
    }
}
