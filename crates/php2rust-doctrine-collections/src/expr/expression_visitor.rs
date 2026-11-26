//! Upstream: doctrine/collections src/Expr/ExpressionVisitor.php

use super::{Comparison, CompositeExpression, Expression, Value};

/// `ExpressionVisitor`: walks an expression tree. PHP's methods return
/// `mixed`; here each visitor names what it returns (`Output`).
pub trait ExpressionVisitor {
    type Output;

    /// `walkComparison($comparison)`.
    fn walk_comparison(&mut self, comparison: &Comparison) -> Self::Output;

    /// `walkValue($value)`.
    fn walk_value(&mut self, value: &Value) -> Self::Output;

    /// `walkCompositeExpression($expr)`.
    fn walk_composite_expression(&mut self, expr: &CompositeExpression) -> Self::Output;

    /// `dispatch($expr)`: `$expr->visit($this)`.
    fn dispatch(&mut self, expr: &Expression) -> Self::Output {
        expr.visit(self)
    }
}
