//! Upstream: doctrine/collections src/Expr/CompositeExpression.php

use php_value::Thrown;

use super::{Expression, ExpressionVisitor};

/// `CompositeExpression`: `AND`, `OR` or `NOT` over child expressions.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeExpression {
    type_: String,
    expressions: Vec<Expression>,
}

impl CompositeExpression {
    pub const TYPE_AND: &'static str = "AND";
    pub const TYPE_OR: &'static str = "OR";
    pub const TYPE_NOT: &'static str = "NOT";

    /// `new CompositeExpression($type, $expressions)`: a `RuntimeException`
    /// for a `Value` child, or for a `NOT` without exactly one child. (PHP
    /// also rejects a child that is not an `Expression`; the type rules it
    /// out here.)
    pub fn new(
        type_: impl Into<String>,
        expressions: impl IntoIterator<Item = Expression>,
    ) -> Result<Self, Thrown> {
        let type_ = type_.into();
        let mut validated = Vec::new();
        for expr in expressions {
            if matches!(expr, Expression::Value(_)) {
                return Err(Thrown::new(
                    "RuntimeException",
                    "Values are not supported expressions as children of and/or expressions.",
                    0,
                ));
            }
            validated.push(expr);
        }
        if type_ == Self::TYPE_NOT && validated.len() != 1 {
            return Err(Thrown::new(
                "RuntimeException",
                "Not expression only allows one expression as child.",
                0,
            ));
        }
        Ok(Self {
            type_,
            expressions: validated,
        })
    }

    /// `getExpressionList()`.
    pub fn get_expression_list(&self) -> &[Expression] {
        &self.expressions
    }

    /// `getType()`.
    pub fn get_type(&self) -> &str {
        &self.type_
    }

    /// `visit($visitor)`: `$visitor->walkCompositeExpression($this)`.
    pub fn visit<V: ExpressionVisitor + ?Sized>(&self, visitor: &mut V) -> V::Output {
        visitor.walk_composite_expression(self)
    }
}
