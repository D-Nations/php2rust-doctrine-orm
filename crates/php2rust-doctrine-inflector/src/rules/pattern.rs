//! Upstream: doctrine/inflector src/Rules/Pattern.php

use php_regex::{Pcre, PcreError};
use php_value::PhpString;

/// A compiled pattern: `None` for one PHP rejects (`preg_match()` warns
/// and returns `false`).
pub(crate) fn compile(regex: &[u8]) -> Result<Option<Pcre>, PcreError> {
    match Pcre::new(regex) {
        Ok(pcre) => Ok(Some(pcre)),
        Err(PcreError::Compile(_)) => Ok(None),
        Err(error @ PcreError::Unsupported(_)) => Err(error),
    }
}

/// `Doctrine\Inflector\Rules\Pattern`.
///
/// `new()` compiles the regex at once and fails only for one `php_regex`
/// does not translate.
#[allow(clippy::struct_field_names, reason = "upstream's property names")]
#[derive(Debug, Clone)]
pub struct Pattern {
    pattern: PhpString,
    regex: PhpString,
    compiled: Option<Pcre>,
}

impl Pattern {
    pub fn new(pattern: impl Into<PhpString>) -> Result<Self, PcreError> {
        let pattern = pattern.into();

        let regex = if pattern.as_bytes().first() == Some(&b'/') {
            pattern.clone()
        } else {
            let mut regex = b"/".to_vec();
            regex.extend_from_slice(pattern.as_bytes());
            regex.extend_from_slice(b"/i");
            PhpString::from(regex)
        };
        let compiled = compile(regex.as_bytes())?;

        Ok(Self {
            pattern,
            regex,
            compiled,
        })
    }

    pub fn get_pattern(&self) -> &PhpString {
        &self.pattern
    }

    pub fn get_regex(&self) -> &PhpString {
        &self.regex
    }

    pub fn matches(&self, word: &[u8]) -> bool {
        self.compiled
            .as_ref()
            .is_some_and(|regex| regex.is_match(word) == Ok(true))
    }

    pub(crate) fn compiled(&self) -> Option<&Pcre> {
        self.compiled.as_ref()
    }
}
