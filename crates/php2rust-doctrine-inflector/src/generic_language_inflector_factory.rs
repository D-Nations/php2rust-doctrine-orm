//! Upstream: doctrine/inflector src/GenericLanguageInflectorFactory.php

use std::sync::Arc;

use crate::cached_word_inflector::CachedWordInflector;
use crate::error::InflectorError;
use crate::inflector::Inflector;
use crate::rules::Ruleset;
use crate::ruleset_inflector::RulesetInflector;

/// `Doctrine\Inflector\GenericLanguageInflectorFactory`: the state and the
/// final methods; a language's factory gives its two rulesets to `new()`
/// (PHP's abstract `getSingularRuleset()` / `getPluralRuleset()`) and
/// delegates [`crate::LanguageInflectorFactory`] to it.
#[derive(Clone)]
pub struct GenericLanguageInflectorFactory {
    singular_rulesets: Vec<Ruleset>,
    plural_rulesets: Vec<Ruleset>,
}

impl GenericLanguageInflectorFactory {
    pub fn new(singular_ruleset: Ruleset, plural_ruleset: Ruleset) -> Self {
        Self {
            singular_rulesets: vec![singular_ruleset],
            plural_rulesets: vec![plural_ruleset],
        }
    }

    /// `build()`. With no ruleset left (after a reset without new rules)
    /// PHP's `RulesetInflector` constructor throws an `ArgumentCountError`;
    /// its message here leaves out PHP's file and line.
    pub fn build(&self) -> Result<Inflector, InflectorError> {
        Ok(Inflector::new(
            Arc::new(CachedWordInflector::new(Arc::new(ruleset_inflector(
                &self.singular_rulesets,
            )?))),
            Arc::new(CachedWordInflector::new(Arc::new(ruleset_inflector(
                &self.plural_rulesets,
            )?))),
        ))
    }

    pub fn with_singular_rules(&mut self, singular_rules: Option<Ruleset>, reset: bool) {
        if reset {
            self.singular_rulesets = Vec::new();
        }

        if let Some(singular_rules) = singular_rules {
            self.singular_rulesets.insert(0, singular_rules);
        }
    }

    pub fn with_plural_rules(&mut self, plural_rules: Option<Ruleset>, reset: bool) {
        if reset {
            self.plural_rulesets = Vec::new();
        }

        if let Some(plural_rules) = plural_rules {
            self.plural_rulesets.insert(0, plural_rules);
        }
    }
}

fn ruleset_inflector(rulesets: &[Ruleset]) -> Result<RulesetInflector, InflectorError> {
    let Some((first, rest)) = rulesets.split_first() else {
        return Err(InflectorError::thrown(
            "ArgumentCountError",
            "Too few arguments to function Doctrine\\Inflector\\RulesetInflector::__construct(), 0 passed and at least 1 expected",
        ));
    };
    Ok(RulesetInflector::new(first.clone(), rest.iter().cloned()))
}

/// A language's `InflectorFactory`: a newtype over
/// [`GenericLanguageInflectorFactory`] built from its `Rules`.
macro_rules! language_inflector_factory {
    ($upstream:literal) => {
        use crate::error::InflectorError;
        use crate::generic_language_inflector_factory::GenericLanguageInflectorFactory;
        use crate::inflector::Inflector;
        use crate::language_inflector_factory::LanguageInflectorFactory;
        use crate::rules::Ruleset;

        use super::rules::Rules;

        #[doc = concat!("`", $upstream, "`.")]
        #[derive(Clone)]
        pub struct InflectorFactory(GenericLanguageInflectorFactory);

        impl InflectorFactory {
            pub fn new() -> Result<Self, InflectorError> {
                Ok(Self(GenericLanguageInflectorFactory::new(
                    Self::get_singular_ruleset()?,
                    Self::get_plural_ruleset()?,
                )))
            }

            fn get_singular_ruleset() -> Result<Ruleset, InflectorError> {
                Ok(Rules::get_singular_ruleset()?)
            }

            fn get_plural_ruleset() -> Result<Ruleset, InflectorError> {
                Ok(Rules::get_plural_ruleset()?)
            }
        }

        impl LanguageInflectorFactory for InflectorFactory {
            fn with_singular_rules(
                &mut self,
                singular_rules: Option<Ruleset>,
                reset: bool,
            ) -> &mut dyn LanguageInflectorFactory {
                self.0.with_singular_rules(singular_rules, reset);
                self
            }

            fn with_plural_rules(
                &mut self,
                plural_rules: Option<Ruleset>,
                reset: bool,
            ) -> &mut dyn LanguageInflectorFactory {
                self.0.with_plural_rules(plural_rules, reset);
                self
            }

            fn build(&self) -> Result<Inflector, InflectorError> {
                self.0.build()
            }
        }
    };
}

pub(crate) use language_inflector_factory;
