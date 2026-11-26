//! Upstream: doctrine/collections src/Expr/Comparison.php

use super::{ExpressionVisitor, Value};

/// `Comparison`: `field operator value`.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    field: String,
    op: String,
    value: Value,
}

impl Comparison {
    pub const EQ: &'static str = "=";
    pub const NEQ: &'static str = "<>";
    pub const LT: &'static str = "<";
    pub const LTE: &'static str = "<=";
    pub const GT: &'static str = ">";
    pub const GTE: &'static str = ">=";
    /// No difference with `EQ`.
    pub const IS: &'static str = "=";
    pub const IN: &'static str = "IN";
    pub const NIN: &'static str = "NIN";
    pub const CONTAINS: &'static str = "CONTAINS";
    pub const MEMBER_OF: &'static str = "MEMBER_OF";
    pub const STARTS_WITH: &'static str = "STARTS_WITH";
    pub const ENDS_WITH: &'static str = "ENDS_WITH";

    /// `new Comparison($field, $op, $value)`: a PHP value is wrapped in a
    /// `Value`, a `Value` is kept.
    pub fn new(field: impl Into<String>, op: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            field: field.into(),
            op: op.into(),
            value: value.into(),
        }
    }

    /// `getField()`.
    pub fn get_field(&self) -> &str {
        &self.field
    }

    /// `getValue()`.
    pub fn get_value(&self) -> &Value {
        &self.value
    }

    /// `getOperator()`.
    pub fn get_operator(&self) -> &str {
        &self.op
    }

    /// `visit($visitor)`: `$visitor->walkComparison($this)`.
    pub fn visit<V: ExpressionVisitor + ?Sized>(&self, visitor: &mut V) -> V::Output {
        visitor.walk_comparison(self)
    }
}
