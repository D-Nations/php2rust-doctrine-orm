//! What the inflector's operations can fail with.
//!
//! Upstream: none

use std::fmt;

use php_regex::PcreError;
use php_value::Thrown;

/// An inflector error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InflectorError {
    /// What PHP throws (`InvalidArgumentException`, `RuntimeException`,
    /// `ArgumentCountError`), with PHP's message.
    Thrown(Thrown),
    /// A pattern PHP accepts and `php_regex` does not translate.
    Unsupported(PcreError),
}

impl InflectorError {
    pub(crate) fn thrown(class: &str, message: impl Into<php_value::PhpString>) -> Self {
        Self::Thrown(Thrown::new(class, message, 0))
    }
}

impl fmt::Display for InflectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Thrown(thrown) => thrown.fmt(f),
            Self::Unsupported(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for InflectorError {}

impl From<PcreError> for InflectorError {
    fn from(error: PcreError) -> Self {
        Self::Unsupported(error)
    }
}
