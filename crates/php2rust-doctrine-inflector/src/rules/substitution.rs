//! Upstream: doctrine/inflector src/Rules/Substitution.php

use super::word::Word;

/// `Doctrine\Inflector\Rules\Substitution`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitution {
    from: Word,
    to: Word,
}

impl Substitution {
    pub fn new(from: Word, to: Word) -> Self {
        Self { from, to }
    }

    pub fn get_from(&self) -> &Word {
        &self.from
    }

    pub fn get_to(&self) -> &Word {
        &self.to
    }
}
