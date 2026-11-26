//! `InvalidArgumentException`.
//!
//! Upstream: doctrine/instantiator src/Doctrine/Instantiator/Exception/InvalidArgumentException.php

use std::fmt;

use php2rust_doctrine_persistence::php_reflection::{
    ReflectionClass, interface_exists, trait_exists,
};

use super::ExceptionInterface;

/// Exception for invalid arguments provided to the instantiator
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidArgumentException {
    message: String,
}

impl InvalidArgumentException {
    pub const CLASS: &'static str = "Doctrine\\Instantiator\\Exception\\InvalidArgumentException";

    pub fn from_non_existing_class(class_name: &str) -> Self {
        crate::declare_classes();
        if interface_exists(class_name) {
            return Self {
                message: format!(
                    "The provided type \"{class_name}\" is an interface, and cannot be instantiated"
                ),
            };
        }
        if trait_exists(class_name) {
            return Self {
                message: format!(
                    "The provided type \"{class_name}\" is a trait, and cannot be instantiated"
                ),
            };
        }
        Self {
            message: format!("The provided class \"{class_name}\" does not exist"),
        }
    }

    pub fn from_abstract_class(reflection_class: &ReflectionClass) -> Self {
        Self {
            message: format!(
                "The provided class \"{}\" is abstract, and cannot be instantiated",
                reflection_class.get_name()
            ),
        }
    }

    pub fn from_enum(class_name: &str) -> Self {
        Self {
            message: format!(
                "The provided class \"{class_name}\" is an enum, and cannot be instantiated"
            ),
        }
    }
}

impl ExceptionInterface for InvalidArgumentException {
    fn class(&self) -> &'static str {
        Self::CLASS
    }

    fn get_message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for InvalidArgumentException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for InvalidArgumentException {}
