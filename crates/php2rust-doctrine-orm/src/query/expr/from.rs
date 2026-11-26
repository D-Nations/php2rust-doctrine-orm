//! `Expr\From`.
//!
//! Upstream: doctrine/orm src/Query/Expr/From.php

use php_value::{PhpString, Thrown, Value};

use super::php::{self, expression_object};

/// Expression class for DQL from.
#[allow(clippy::struct_field_names, reason = "upstream's property names")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct From {
    from: String,
    alias: String,
    index_by: Option<String>,
}

impl From {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\Expr\\From";

    pub fn new(from: impl Into<String>, alias: impl Into<String>, index_by: Option<&str>) -> Self {
        Self {
            from: from.into(),
            alias: alias.into(),
            index_by: index_by.map(str::to_owned),
        }
    }

    pub fn get_from(&self) -> &str {
        &self.from
    }

    pub fn get_alias(&self) -> &str {
        &self.alias
    }

    pub fn get_index_by(&self) -> Option<&str> {
        self.index_by.as_deref()
    }

    fn properties_in_order(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("from", Value::string(self.from.as_str())),
            ("alias", Value::string(self.alias.as_str())),
            ("indexBy", php::nullable(self.index_by.as_deref())),
        ]
    }

    /// `__toString()`.
    pub fn try_to_string(&self) -> Result<PhpString, Thrown> {
        let mut out = format!("{} {}", self.from, self.alias);
        if let Some(index_by) = self
            .index_by
            .as_deref()
            .filter(|i| php::string_is_truthy(i))
        {
            out.push_str(" INDEX BY ");
            out.push_str(index_by);
        }
        Ok(PhpString::from(out))
    }
}

expression_object!(From);
