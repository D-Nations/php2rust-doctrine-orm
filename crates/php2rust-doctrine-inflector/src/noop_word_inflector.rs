//! Upstream: doctrine/inflector src/NoopWordInflector.php

use php_value::PhpString;

use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\NoopWordInflector`: the word as it is.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopWordInflector;

impl WordInflector for NoopWordInflector {
    fn inflect(&self, word: &[u8]) -> PhpString {
        PhpString::from(word)
    }
}
