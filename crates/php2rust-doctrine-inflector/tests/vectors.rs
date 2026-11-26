//! doctrine/inflector as PHP 8.4.27 runs it (`tests/data/inflector.json`,
//! `oracle/recorders/inflector.php`): `singularize()` and `pluralize()` of
//! 2,712 words (every provider word as given, `ucfirst()`ed and upper
//! case) in all eight languages, and `tableize()`, `classify()`,
//! `camelize()`, `capitalize()`, `seemsUtf8()`, `unaccent()` and
//! `urlize()` of 1,358 strings (accents, Greek sigma, Turkish I,
//! ISO-8859-1 bytes, invalid UTF-8, drawn strings) with PHP's exceptions.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

mod support;

use php_value::PhpString;
use php2rust_doctrine_inflector::{InflectorError, InflectorFactory, Language};
use serde_json::{Value as Json, json};
use support::{data, hex, text};

const LANGUAGES: [(&str, &str); 8] = [
    ("English", Language::ENGLISH),
    ("Esperanto", Language::ESPERANTO),
    ("French", Language::FRENCH),
    ("Italian", Language::ITALIAN),
    ("NorwegianBokmal", Language::NORWEGIAN_BOKMAL),
    ("Portuguese", Language::PORTUGUESE),
    ("Spanish", Language::SPANISH),
    ("Turkish", Language::TURKISH),
];

#[test]
fn words_inflect_as_php() {
    let words: Vec<Vec<u8>> = data()["words"]
        .as_array()
        .unwrap()
        .iter()
        .map(hex)
        .collect();
    assert_eq!(2712, words.len());
    for (name, language) in LANGUAGES {
        let inflector = InflectorFactory::create_for_language(language)
            .unwrap()
            .build()
            .unwrap();
        let expected = &data()["inflected"][name];
        for (i, word) in words.iter().enumerate() {
            assert_eq!(
                text(&hex(&expected["singularize"][i])),
                text(inflector.singularize(word).as_bytes()),
                "{name} singularize {:?}",
                text(word)
            );
            assert_eq!(
                text(&hex(&expected["pluralize"][i])),
                text(inflector.pluralize(word).as_bytes()),
                "{name} pluralize {:?}",
                text(word)
            );
        }
    }
}

fn encode(result: Result<PhpString, InflectorError>) -> Json {
    match result {
        Ok(s) => json!(hex_of(s.as_bytes())),
        Err(InflectorError::Thrown(thrown)) => {
            json!([thrown.class, hex_of(thrown.message.as_bytes())])
        }
        Err(InflectorError::Unsupported(error)) => json!(error.to_string()),
    }
}

fn hex_of(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

#[test]
fn strings_as_php() {
    let inflector = InflectorFactory::create().unwrap().build().unwrap();
    let strings = data()["strings"].as_array().unwrap();
    assert_eq!(1358, strings.len());
    for case in strings {
        let s = hex(&case["input"]);
        let input = format!("{:?}", text(&s));
        assert_eq!(
            case["tableize"],
            encode(inflector.tableize(&s)),
            "tableize {input}"
        );
        assert_eq!(
            case["classify"],
            encode(Ok(inflector.classify(&s))),
            "classify {input}"
        );
        assert_eq!(
            case["camelize"],
            encode(Ok(inflector.camelize(&s))),
            "camelize {input}"
        );
        assert_eq!(
            case["capitalize"],
            encode(Ok(inflector.capitalize(&s, None))),
            "capitalize {input}"
        );
        assert_eq!(
            case["capitalize_custom"],
            encode(Ok(inflector.capitalize(&s, Some(b"-_ ")))),
            "capitalize custom {input}"
        );
        assert_eq!(
            case["seems_utf8"],
            json!(inflector.seems_utf8(&s)),
            "seemsUtf8 {input}"
        );
        assert_eq!(
            case["unaccent"],
            encode(Ok(inflector.unaccent(&s))),
            "unaccent {input}"
        );
        assert_eq!(
            case["urlize"],
            encode(inflector.urlize(&s)),
            "urlize {input}"
        );
    }
}
