//! Upstream: doctrine/inflector src/InflectorFactory.php

use crate::error::InflectorError;
use crate::language::Language;
use crate::language_inflector_factory::LanguageInflectorFactory;
use crate::rules::{
    english, esperanto, french, italian, norwegian_bokmal, portuguese, spanish, turkish,
};

/// `Doctrine\Inflector\InflectorFactory`.
pub struct InflectorFactory;

impl InflectorFactory {
    pub fn create() -> Result<Box<dyn LanguageInflectorFactory>, InflectorError> {
        Self::create_for_language(Language::ENGLISH)
    }

    pub fn create_for_language(
        language: &str,
    ) -> Result<Box<dyn LanguageInflectorFactory>, InflectorError> {
        Ok(match language {
            Language::ENGLISH => Box::new(english::InflectorFactory::new()?),
            Language::ESPERANTO => Box::new(esperanto::InflectorFactory::new()?),
            Language::FRENCH => Box::new(french::InflectorFactory::new()?),
            Language::ITALIAN => Box::new(italian::InflectorFactory::new()?),
            Language::NORWEGIAN_BOKMAL => Box::new(norwegian_bokmal::InflectorFactory::new()?),
            Language::PORTUGUESE => Box::new(portuguese::InflectorFactory::new()?),
            Language::SPANISH => Box::new(spanish::InflectorFactory::new()?),
            Language::TURKISH => Box::new(turkish::InflectorFactory::new()?),
            _ => {
                return Err(InflectorError::thrown(
                    "InvalidArgumentException",
                    format!("Language \"{language}\" is not supported."),
                ));
            }
        })
    }
}
