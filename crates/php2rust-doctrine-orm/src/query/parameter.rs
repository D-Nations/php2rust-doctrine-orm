//! `Parameter`.
//!
//! Upstream: doctrine/orm src/Query/Parameter.php

use php_value::Value;
use php2rust_doctrine_dbal::WrapperParameterType;

use super::ParameterTypeInferer;

/// `int|string $name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterName {
    Int(i64),
    String(String),
}

impl From<i64> for ParameterName {
    fn from(name: i64) -> Self {
        Self::Int(name)
    }
}

impl From<&str> for ParameterName {
    fn from(name: &str) -> Self {
        Self::String(name.to_owned())
    }
}

impl From<String> for ParameterName {
    fn from(name: String) -> Self {
        Self::String(name)
    }
}

/// Defines a Query Parameter.
#[derive(Debug, Clone)]
pub struct Parameter {
    /// The parameter name.
    name: String,
    /// The parameter value.
    value: Value,
    /// The parameter type.
    type_: WrapperParameterType,
    /// Whether the parameter type was explicitly specified or not
    type_specified: bool,
}

impl Parameter {
    /// Returns the internal representation of a parameter name.
    pub fn normalize_name(name: impl Into<ParameterName>) -> String {
        match name.into() {
            ParameterName::Int(i) => i.to_string(),
            ParameterName::String(s) => s.trim_matches(':').to_owned(),
        }
    }

    /// `new Parameter($name, $value, $type)`: a `None` type is inferred
    /// from the value.
    pub fn new(
        name: impl Into<ParameterName>,
        value: impl Into<Value>,
        type_: Option<WrapperParameterType>,
    ) -> Self {
        let type_specified = type_.is_some();
        let value = value.into();
        let type_ = Self::chosen_type(&value, type_);
        Self {
            name: Self::normalize_name(name),
            value,
            type_,
            type_specified,
        }
    }

    /// Retrieves the Parameter name.
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Retrieves the Parameter value.
    pub fn get_value(&self) -> &Value {
        &self.value
    }

    /// Retrieves the Parameter type.
    pub fn get_type(&self) -> &WrapperParameterType {
        &self.type_
    }

    /// Defines the Parameter value.
    pub fn set_value(&mut self, value: impl Into<Value>, type_: Option<WrapperParameterType>) {
        self.value = value.into();
        self.type_ = Self::chosen_type(&self.value, type_);
    }

    pub fn type_was_specified(&self) -> bool {
        self.type_specified
    }

    /// `$type ?: ParameterTypeInferer::inferType($value)`: a falsy type
    /// (the names `""` and `"0"`) is inferred too.
    fn chosen_type(value: &Value, type_: Option<WrapperParameterType>) -> WrapperParameterType {
        match type_ {
            Some(WrapperParameterType::Name(name))
                if name.as_bytes().is_empty() || name.as_bytes() == b"0" =>
            {
                ParameterTypeInferer::infer_type(value)
            }
            Some(type_) => type_,
            None => ParameterTypeInferer::infer_type(value),
        }
    }
}
