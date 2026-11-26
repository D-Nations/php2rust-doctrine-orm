//! Upstream: doctrine/inflector src/Rules/Italian/Uninflected.php

use php_regex::PcreError;

use crate::rules::Pattern;

/// `Doctrine\Inflector\Rules\Italian\Uninflected`.
pub struct Uninflected;

impl Uninflected {
    pub fn get_singular() -> Result<Vec<Pattern>, PcreError> {
        Self::get_default()
    }

    pub fn get_plural() -> Result<Vec<Pattern>, PcreError> {
        Self::get_default()
    }

    fn get_default() -> Result<Vec<Pattern>, PcreError> {
        // Invariable words (same form in singular and plural)
        let invariables: &[&str] = &[
            "alpaca",
            "auto",
            "bar",
            "blu",
            "boia",
            "boomerang",
            "brindisi",
            "campus",
            "computer",
            "crisi",
            "crocevia",
            "dopocena",
            "film",
            "foto",
            "fuchsia",
            "gnu",
            "gorilla",
            "gru",
            "iguana",
            "kamikaze",
            "karaoke",
            "koala",
            "lama",
            "menu",
            "metropoli",
            "moto",
            "opossum",
            "panda",
            "quiz",
            "radio",
            "re",
            "scacciapensieri",
            "serie",
            "smartphone",
            "sosia",
            "sottoscala",
            "specie",
            "sport",
            "tablet",
            "taxi",
            "vaglia",
            "virtù",
            "virus",
            "yogurt",
            "foto",
            "fuchsia",
        ];

        invariables.iter().map(|word| Pattern::new(*word)).collect()
    }
}
