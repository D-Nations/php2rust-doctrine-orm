//! `Expr\Func`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Func.php

use php_value::{PhpArray, PhpString, Thrown, Value};

use super::php::{self, expression_object};

/// Expression class for generating DQL functions.
#[derive(Debug, Clone)]
pub struct Func {
    name: String,
    arguments: PhpArray,
}

impl Func {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\Expr\\Func";

    /// Creates a function, with the given argument (`(array) $arguments`).
    pub fn new(name: impl Into<String>, arguments: impl Into<Value>) -> Self {
        Self {
            name: name.into(),
            arguments: php::to_array(&arguments.into()),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_arguments(&self) -> &PhpArray {
        &self.arguments
    }

    fn properties_in_order(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("arguments", Value::Array(self.arguments.clone())),
            ("name", Value::from(self.name.as_str())),
        ]
    }

    /// `__toString()`.
    pub fn try_to_string(&self) -> Result<PhpString, Thrown> {
        Ok(php::concat(&[
            self.name.as_bytes(),
            b"(",
            php::implode(", ", self.arguments.values())?.as_bytes(),
            b")",
        ]))
    }
}

expression_object!(Func);
