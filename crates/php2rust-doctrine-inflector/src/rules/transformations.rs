//! Upstream: doctrine/inflector src/Rules/Transformations.php

use php_value::PhpString;

use super::transformation::Transformation;
use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\Rules\Transformations`: the first transformation
/// whose pattern matches.
#[derive(Debug, Clone, Default)]
pub struct Transformations {
    transformations: Vec<Transformation>,
}

impl Transformations {
    pub fn new(transformations: impl IntoIterator<Item = Transformation>) -> Self {
        Self {
            transformations: transformations.into_iter().collect(),
        }
    }
}

impl WordInflector for Transformations {
    fn inflect(&self, word: &[u8]) -> PhpString {
        for transformation in &self.transformations {
            if transformation.get_pattern().matches(word) {
                return transformation.inflect(word);
            }
        }

        PhpString::from(word)
    }
}
