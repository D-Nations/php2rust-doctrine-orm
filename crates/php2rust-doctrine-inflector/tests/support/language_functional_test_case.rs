//! `LanguageFunctionalTestCase`: each language's sample words singularized
//! and pluralized. A language's test file calls these with its provider.
//!
//! Upstream: doctrine/inflector tests/Rules/LanguageFunctionalTestCase.php

#![allow(dead_code, reason = "each test file uses some of the helpers")]

use php2rust_doctrine_inflector::{Inflector, InflectorFactory};

use super::{provider, text};

pub fn create_inflector(language: &str) -> Inflector {
    InflectorFactory::create_for_language(language)
        .unwrap()
        .build()
        .unwrap()
}

pub fn test_inflecting_singulars(language: &str, test: &str) {
    let inflector = create_inflector(language);
    for row in provider(&format!("{test}::dataSampleWords")) {
        let (singular, plural) = (&row[0], &row[1]);
        assert_eq!(
            text(singular),
            text(inflector.singularize(plural).as_bytes()),
            "'{}' should be singularized to '{}'",
            text(plural),
            text(singular)
        );
    }
}

pub fn test_inflecting_plurals(language: &str, test: &str) {
    let inflector = create_inflector(language);
    for row in provider(&format!("{test}::dataSampleWords")) {
        let (singular, plural) = (&row[0], &row[1]);
        assert_eq!(
            text(plural),
            text(inflector.pluralize(singular).as_bytes()),
            "'{}' should be pluralized to '{}'",
            text(singular),
            text(plural)
        );
    }
}
