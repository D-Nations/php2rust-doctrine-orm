//! Upstream: doctrine/inflector src/Rules/Transformation.php

use php_value::PhpString;

use super::pattern::Pattern;
use crate::php::preg_replace;
use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\Rules\Transformation`.
#[derive(Debug, Clone)]
pub struct Transformation {
    pattern: Pattern,
    replacement: PhpString,
}

impl Transformation {
    pub fn new(pattern: Pattern, replacement: impl Into<PhpString>) -> Self {
        Self {
            pattern,
            replacement: replacement.into(),
        }
    }

    pub fn get_pattern(&self) -> &Pattern {
        &self.pattern
    }

    pub fn get_replacement(&self) -> &PhpString {
        &self.replacement
    }
}

impl WordInflector for Transformation {
    /// `(string) preg_replace()`: `""` where PHP's is `null`.
    fn inflect(&self, word: &[u8]) -> PhpString {
        PhpString::from(
            preg_replace(self.pattern.compiled(), self.replacement.as_bytes(), word)
                .unwrap_or_default(),
        )
    }
}
