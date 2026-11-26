//! `Expr\Join`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Join.php

use php_value::{PhpString, Thrown, Value};

use super::php::{self, expression_object};

/// Expression class for DQL join.
#[allow(clippy::struct_field_names, reason = "upstream's property names")]
#[derive(Debug, Clone)]
pub struct Join {
    join_type: String,
    join: String,
    alias: Option<String>,
    condition_type: Option<String>,
    condition: Value,
    index_by: Option<String>,
}

impl Join {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\Expr\\Join";

    pub const INNER_JOIN: &'static str = "INNER";
    pub const LEFT_JOIN: &'static str = "LEFT";

    pub const ON: &'static str = "ON";
    pub const WITH: &'static str = "WITH";

    /// `$condition` is a string, a `Comparison`, a `Composite` (`Andx`,
    /// `Orx`), a `Func` or `null`.
    pub fn new(
        join_type: impl Into<String>,
        join: impl Into<String>,
        alias: Option<&str>,
        condition_type: Option<&str>,
        condition: impl Into<Value>,
        index_by: Option<&str>,
    ) -> Self {
        Self {
            join_type: join_type.into(),
            join: join.into(),
            alias: alias.map(str::to_owned),
            condition_type: condition_type.map(str::to_owned),
            condition: condition.into(),
            index_by: index_by.map(str::to_owned),
        }
    }

    pub fn get_join_type(&self) -> &str {
        &self.join_type
    }

    pub fn get_join(&self) -> &str {
        &self.join
    }

    pub fn get_alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    pub fn get_condition_type(&self) -> Option<&str> {
        self.condition_type.as_deref()
    }

    pub fn get_condition(&self) -> &Value {
        &self.condition
    }

    pub fn get_index_by(&self) -> Option<&str> {
        self.index_by.as_deref()
    }

    fn properties_in_order(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("joinType", Value::from(self.join_type.as_str())),
            ("join", Value::from(self.join.as_str())),
            ("alias", php::nullable(self.alias.as_deref())),
            ("conditionType", php::nullable(self.condition_type.as_deref())),
            ("condition", self.condition.clone()),
            ("indexBy", php::nullable(self.index_by.as_deref())),
        ]
    }

    /// `__toString()`.
    pub fn try_to_string(&self) -> Result<PhpString, Thrown> {
        let mut out = Vec::new();
        out.extend_from_slice(self.join_type.to_ascii_uppercase().as_bytes());
        out.extend_from_slice(b" JOIN ");
        out.extend_from_slice(self.join.as_bytes());
        if let Some(alias) = truthy(self.alias.as_ref()) {
            out.push(b' ');
            out.extend_from_slice(alias.as_bytes());
        }
        if let Some(index_by) = truthy(self.index_by.as_ref()) {
            out.extend_from_slice(b" INDEX BY ");
            out.extend_from_slice(index_by.as_bytes());
        }
        if self.condition.is_truthy() {
            // strtoupper() takes no null under strict types.
            let condition_type = self.condition_type.as_deref().ok_or_else(|| {
                Thrown::new(
                    "TypeError",
                    "strtoupper(): Argument #1 ($string) must be of type string, null given",
                    0,
                )
            })?;
            out.push(b' ');
            out.extend_from_slice(condition_type.to_ascii_uppercase().as_bytes());
            out.push(b' ');
            out.extend_from_slice(php::to_string(&self.condition)?.as_bytes());
        }
        Ok(PhpString::from(out))
    }
}

fn truthy(s: Option<&String>) -> Option<&str> {
    s.map(String::as_str).filter(|s| php::string_is_truthy(s))
}

expression_object!(Join);
