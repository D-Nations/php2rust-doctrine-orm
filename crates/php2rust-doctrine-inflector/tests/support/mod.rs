//! Test doubles for PHPUnit's mocks, and the upstream tests' data
//! providers as PHP 8.4.27 runs them (`tests/data/inflector.json`,
//! oracle/recorders/inflector.php).
//!
//! Upstream: none

#![allow(dead_code, reason = "each test file uses some of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::{Mutex, OnceLock};

use php_value::PhpString;
use php2rust_doctrine_inflector::WordInflector;
use php2rust_doctrine_inflector::rules::WordMatcher;
use serde_json::Value as Json;

pub fn data() -> &'static Json {
    static DATA: OnceLock<Json> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("../data/inflector.json")).unwrap())
}

pub fn hex(json: &Json) -> Vec<u8> {
    let text = json.as_str().unwrap().as_bytes();
    text.chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

/// The rows of a data provider, e.g. `EnglishFunctionalTest::dataSampleWords`.
pub fn provider(name: &str) -> Vec<Vec<Vec<u8>>> {
    data()["providers"][name]
        .as_array()
        .unwrap_or_else(|| panic!("no provider {name}"))
        .iter()
        .map(|row| row.as_array().unwrap().iter().map(hex).collect())
        .collect()
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `$this->createMock(WordInflector::class)` (or of a class implementing
/// it): `inflect()` returns what `will_return` sets for the expected
/// argument (`with()`), else `''`, and records its calls.
#[derive(Default)]
pub struct MockWordInflector {
    with: Option<(Vec<u8>, PhpString)>,
    pub calls: Mutex<Vec<Vec<u8>>>,
}

impl MockWordInflector {
    pub fn returning(with: &str, value: &str) -> Self {
        Self {
            with: Some((with.as_bytes().to_vec(), PhpString::from(value))),
            calls: Mutex::default(),
        }
    }

    /// `expects(self::once())->...->with('in')`, `expects(self::never())`.
    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().iter().map(|c| text(c)).collect()
    }
}

impl WordInflector for MockWordInflector {
    fn inflect(&self, word: &[u8]) -> PhpString {
        self.calls.lock().unwrap().push(word.to_vec());
        match &self.with {
            Some((with, value)) => {
                assert_eq!(text(with), text(word), "unexpected argument");
                value.clone()
            }
            None => PhpString::default(),
        }
    }
}

/// `$this->createMock(Patterns::class)`: `matches()` returns what is set
/// (else `false`), and records its calls.
#[derive(Default)]
pub struct MockWordMatcher {
    with: Option<(Vec<u8>, bool)>,
    pub calls: Mutex<Vec<Vec<u8>>>,
}

impl MockWordMatcher {
    pub fn returning(with: &str, value: bool) -> Self {
        Self {
            with: Some((with.as_bytes().to_vec(), value)),
            calls: Mutex::default(),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().iter().map(|c| text(c)).collect()
    }
}

impl WordMatcher for MockWordMatcher {
    fn matches(&self, word: &[u8]) -> bool {
        self.calls.lock().unwrap().push(word.to_vec());
        match &self.with {
            Some((with, value)) => {
                assert_eq!(text(with), text(word), "unexpected argument");
                *value
            }
            None => false,
        }
    }
}
pub mod language_functional_test_case;
