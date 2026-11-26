//! Upstream: doctrine/inflector src/Rules/French/Uninflected.php

use php_regex::PcreError;

use crate::rules::Pattern;

/// `Doctrine\Inflector\Rules\French\Uninflected`.
pub struct Uninflected;

impl Uninflected {
    pub fn get_singular() -> Result<Vec<Pattern>, PcreError> {
        let mut patterns = Self::get_default()?;

        patterns.extend([Pattern::new("bois")?, Pattern::new("mas")?]);

        Ok(patterns)
    }

    pub fn get_plural() -> Result<Vec<Pattern>, PcreError> {
        Self::get_default()
    }

    fn get_default() -> Result<Vec<Pattern>, PcreError> {
        Ok(vec![Pattern::new("")?])
    }
}
