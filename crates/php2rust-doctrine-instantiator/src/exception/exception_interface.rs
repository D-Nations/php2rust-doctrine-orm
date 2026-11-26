//! `ExceptionInterface`.
//!
//! Upstream: doctrine/instantiator src/Doctrine/Instantiator/Exception/ExceptionInterface.php

use php_value::Thrown;

/// Base exception marker interface for the instantiator component
pub trait ExceptionInterface: std::error::Error {
    /// The exception's class name.
    fn class(&self) -> &'static str;

    /// `getMessage()`.
    fn get_message(&self) -> &str;

    /// `getCode()`.
    fn get_code(&self) -> i64 {
        0
    }

    /// `getPrevious()`.
    fn get_previous(&self) -> Option<&Thrown> {
        None
    }

    /// The exception as `php_value` carries a thrown object.
    fn to_thrown(&self) -> Thrown {
        Thrown::new(self.class(), self.get_message(), self.get_code())
    }
}
