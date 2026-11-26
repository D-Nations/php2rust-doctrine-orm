//! Upstream: doctrine/inflector src/LanguageInflectorFactory.php

use std::any::Any;

use crate::error::InflectorError;
use crate::inflector::Inflector;
use crate::rules::Ruleset;

/// `Doctrine\Inflector\LanguageInflectorFactory`.
pub trait LanguageInflectorFactory: Any {
    /// Applies custom rules for singularisation
    ///
    /// `reset`: if true, will unset default inflections for all new rules
    fn with_singular_rules(
        &mut self,
        singular_rules: Option<Ruleset>,
        reset: bool,
    ) -> &mut dyn LanguageInflectorFactory;

    /// Applies custom rules for pluralisation
    ///
    /// `reset`: if true, will unset default inflections for all new rules
    fn with_plural_rules(
        &mut self,
        plural_rules: Option<Ruleset>,
        reset: bool,
    ) -> &mut dyn LanguageInflectorFactory;

    /// Builds the inflector instance with all applicable rules
    fn build(&self) -> Result<Inflector, InflectorError>;
}
