//! Upstream: doctrine/collections src/ExpressionBuilder.php

use php_value::{PhpArray, Thrown, Value as PhpValue};

use crate::expr::{Comparison, CompositeExpression, Expression, Value};

/// `ExpressionBuilder`: builds the expressions of a `Criteria`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExpressionBuilder;

impl ExpressionBuilder {
    pub fn new() -> Self {
        Self
    }

    /// `andX(...$expressions)`: a `RuntimeException` for a `Value`.
    pub fn and_x(
        &self,
        expressions: impl IntoIterator<Item = Expression>,
    ) -> Result<CompositeExpression, Thrown> {
        CompositeExpression::new(CompositeExpression::TYPE_AND, expressions)
    }

    /// `orX(...$expressions)`: a `RuntimeException` for a `Value`.
    pub fn or_x(
        &self,
        expressions: impl IntoIterator<Item = Expression>,
    ) -> Result<CompositeExpression, Thrown> {
        CompositeExpression::new(CompositeExpression::TYPE_OR, expressions)
    }

    /// `not($expression)`: a `RuntimeException` for a `Value`.
    pub fn not(&self, expression: impl Into<Expression>) -> Result<CompositeExpression, Thrown> {
        CompositeExpression::new(CompositeExpression::TYPE_NOT, [expression.into()])
    }

    fn comparison(field: &str, op: &str, value: impl Into<PhpValue>) -> Comparison {
        Comparison::new(field, op, Value::new(value))
    }

    /// `eq($field, $value)`.
    pub fn eq(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::EQ, value)
    }

    /// `gt($field, $value)`.
    pub fn gt(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::GT, value)
    }

    /// `lt($field, $value)`.
    pub fn lt(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::LT, value)
    }

    /// `gte($field, $value)`.
    pub fn gte(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::GTE, value)
    }

    /// `lte($field, $value)`.
    pub fn lte(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::LTE, value)
    }

    /// `neq($field, $value)`.
    pub fn neq(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::NEQ, value)
    }

    /// `isNull($field)`: `EQ` `null`.
    pub fn is_null(&self, field: &str) -> Comparison {
        Self::comparison(field, Comparison::EQ, PhpValue::Null)
    }

    /// `isNotNull($field)`: `NEQ` `null`.
    pub fn is_not_null(&self, field: &str) -> Comparison {
        Self::comparison(field, Comparison::NEQ, PhpValue::Null)
    }

    /// `in($field, $values)`.
    pub fn r#in(&self, field: &str, values: PhpArray) -> Comparison {
        Self::comparison(field, Comparison::IN, values)
    }

    /// `notIn($field, $values)`.
    pub fn not_in(&self, field: &str, values: PhpArray) -> Comparison {
        Self::comparison(field, Comparison::NIN, values)
    }

    /// `contains($field, $value)`.
    pub fn contains(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::CONTAINS, value)
    }

    /// `memberOf($field, $value)`.
    pub fn member_of(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::MEMBER_OF, value)
    }

    /// `startsWith($field, $value)`.
    pub fn starts_with(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::STARTS_WITH, value)
    }

    /// `endsWith($field, $value)`.
    pub fn ends_with(&self, field: &str, value: impl Into<PhpValue>) -> Comparison {
        Self::comparison(field, Comparison::ENDS_WITH, value)
    }
}
