//! Upstream: doctrine/inflector src/Rules/Word.php

use php_value::PhpString;

/// `Doctrine\Inflector\Rules\Word`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    word: PhpString,
}

impl Word {
    pub fn new(word: impl Into<PhpString>) -> Self {
        Self { word: word.into() }
    }

    pub fn get_word(&self) -> &PhpString {
        &self.word
    }
}
