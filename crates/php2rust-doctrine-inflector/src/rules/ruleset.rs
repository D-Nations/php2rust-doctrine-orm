//! Upstream: doctrine/inflector src/Rules/Ruleset.php

use std::sync::Arc;

use super::patterns::{Patterns, WordMatcher};
use super::substitutions::Substitutions;
use super::transformations::Transformations;
use crate::word_inflector::WordInflector;

/// `Doctrine\Inflector\Rules\Ruleset`. Its parts are shared trait objects,
/// so that a test can give doubles, as PHPUnit's mocks of the classes do.
#[derive(Clone)]
pub struct Ruleset {
    regular: Arc<dyn WordInflector>,
    uninflected: Arc<dyn WordMatcher>,
    irregular: Arc<dyn WordInflector>,
}

impl Ruleset {
    pub fn new(regular: Transformations, uninflected: Patterns, irregular: Substitutions) -> Self {
        Self::from_parts(
            Arc::new(regular),
            Arc::new(uninflected),
            Arc::new(irregular),
        )
    }

    /// A ruleset of any regular, uninflected and irregular rules.
    pub fn from_parts(
        regular: Arc<dyn WordInflector>,
        uninflected: Arc<dyn WordMatcher>,
        irregular: Arc<dyn WordInflector>,
    ) -> Self {
        Self {
            regular,
            uninflected,
            irregular,
        }
    }

    pub fn get_regular(&self) -> &Arc<dyn WordInflector> {
        &self.regular
    }

    pub fn get_uninflected(&self) -> &Arc<dyn WordMatcher> {
        &self.uninflected
    }

    pub fn get_irregular(&self) -> &Arc<dyn WordInflector> {
        &self.irregular
    }
}
