//! The expression tree of `Criteria`: `Expression` (an interface in PHP,
//! here an enum of the three implementations upstream ships), and the
//! visitors over it.
//!
//! Upstream: doctrine/collections src/Expr/Expression.php

mod comparison;
mod composite_expression;
mod expression_visitor;
mod value;

pub use comparison::Comparison;
pub use composite_expression::CompositeExpression;
pub use expression_visitor::ExpressionVisitor;
pub use value::Value;

/// `Expression`: a node of a criteria's expression tree. PHP's interface is
/// open to other classes; the package's own are `Comparison`,
/// `CompositeExpression` and `Value`, and those are the cases here.
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Comparison(Comparison),
    Composite(CompositeExpression),
    Value(Value),
}

impl Expression {
    /// `visit($visitor)`: the node's own `walk*()` method of `visitor`.
    pub fn visit<V: ExpressionVisitor + ?Sized>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Comparison(comparison) => comparison.visit(visitor),
            Self::Composite(composite) => composite.visit(visitor),
            Self::Value(value) => value.visit(visitor),
        }
    }
}

impl From<Comparison> for Expression {
    fn from(comparison: Comparison) -> Self {
        Self::Comparison(comparison)
    }
}

impl From<CompositeExpression> for Expression {
    fn from(composite: CompositeExpression) -> Self {
        Self::Composite(composite)
    }
}

impl From<Value> for Expression {
    fn from(value: Value) -> Self {
        Self::Value(value)
    }
}
