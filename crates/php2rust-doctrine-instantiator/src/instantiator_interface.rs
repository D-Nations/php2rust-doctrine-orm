//! `InstantiatorInterface`.
//!
//! Upstream: doctrine/instantiator src/Doctrine/Instantiator/InstantiatorInterface.php

use php_value::Object;

use crate::exception::InstantiatorError;

/// Instantiator provides utility methods to build objects without invoking
/// their constructors.
pub trait InstantiatorInterface {
    /// `instantiate($className)`: a blank instance of `class_name`, or the
    /// [`crate::exception::ExceptionInterface`] exception (or what PHP
    /// itself threw) on failure.
    fn instantiate(&self, class_name: &str) -> Result<Object, InstantiatorError>;
}
