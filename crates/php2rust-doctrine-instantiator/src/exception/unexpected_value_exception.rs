//! `UnexpectedValueException`.
//!
//! Upstream: doctrine/instantiator src/Doctrine/Instantiator/Exception/UnexpectedValueException.php

use std::fmt;

use php_value::Thrown;
use php2rust_doctrine_persistence::php_reflection::ReflectionClass;

use super::ExceptionInterface;

/// Exception for given parameters causing invalid/unexpected state on instantiation
#[derive(Debug, Clone, PartialEq)]
pub struct UnexpectedValueException {
    message: String,
    previous: Thrown,
}

impl UnexpectedValueException {
    pub const CLASS: &'static str = "Doctrine\\Instantiator\\Exception\\UnexpectedValueException";

    /// `$exception` is an `Exception` (not an `Error`), as PHP's parameter
    /// type requires.
    pub fn from_serialization_triggered_exception(
        reflection_class: &ReflectionClass,
        exception: Thrown,
    ) -> Self {
        Self {
            message: format!(
                "An exception was raised while trying to instantiate an instance of \"{}\" via un-serialization",
                reflection_class.get_name()
            ),
            previous: exception,
        }
    }

    pub fn from_unclean_un_serialization(
        reflection_class: &ReflectionClass,
        error_string: &str,
        error_code: i64,
        error_file: &str,
        error_line: i64,
    ) -> Self {
        Self {
            message: format!(
                "Could not produce an instance of \"{}\" via un-serialization, since an error was triggered in file \"{error_file}\" at line \"{error_line}\"",
                reflection_class.get_name()
            ),
            previous: Thrown::new("Exception", error_string, error_code),
        }
    }
}

impl ExceptionInterface for UnexpectedValueException {
    fn class(&self) -> &'static str {
        Self::CLASS
    }

    fn get_message(&self) -> &str {
        &self.message
    }

    fn get_previous(&self) -> Option<&Thrown> {
        Some(&self.previous)
    }
}

impl fmt::Display for UnexpectedValueException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for UnexpectedValueException {}
