//! Upstream: doctrine/inflector src/Rules/NorwegianBokmal/Uninflected.php

use php_regex::PcreError;

use crate::rules::Pattern;

/// `Doctrine\Inflector\Rules\NorwegianBokmal\Uninflected`.
pub struct Uninflected;

impl Uninflected {
    pub fn get_singular() -> Result<Vec<Pattern>, PcreError> {
        Self::get_default()
    }

    pub fn get_plural() -> Result<Vec<Pattern>, PcreError> {
        Self::get_default()
    }

    fn get_default() -> Result<Vec<Pattern>, PcreError> {
        Ok(vec![
            Pattern::new("barn")?,
            Pattern::new("fjell")?,
            Pattern::new("hus")?,
        ])
    }
}
