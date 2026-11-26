//! Port of tests/Expr/CompositeExpressionTest.php at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d. The stubbed expressions are
//! comparisons, and the mocked visitor counts its calls.
//!
//! Upstream: doctrine/collections tests/Expr/CompositeExpressionTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(
    clippy::doc_markdown,
    clippy::disallowed_names,
    reason = "upstream's test, class and field names"
)]
#![allow(non_snake_case)]

use php2rust_doctrine_collections::expr::{
    Comparison, CompositeExpression, Expression, ExpressionVisitor, Value,
};

/// `createStub(Expression::class)`.
fn stub() -> Expression {
    Comparison::new("stub", Comparison::EQ, 1).into()
}

/// `invalidDataProvider()`. The second row, a string among the
/// expressions, is ruled out by the types.
fn invalid_data_provider() -> Vec<(&'static str, Vec<Expression>)> {
    vec![
        (
            CompositeExpression::TYPE_AND,
            vec![Value::new("value").into()],
        ),
        (CompositeExpression::TYPE_NOT, vec![stub(), stub()]),
    ]
}

#[test]
fn testExceptions() {
    for (type_, expressions) in invalid_data_provider() {
        let error = CompositeExpression::new(type_, expressions).unwrap_err();
        assert_eq!("RuntimeException", error.class);
    }
}

#[test]
fn testGetType() {
    let composite_expression = create_composite_expression();

    let expected_type = CompositeExpression::TYPE_AND;
    let actual_type = composite_expression.get_type();

    assert_eq!(expected_type, actual_type);
}

/// `createCompositeExpression()`.
fn create_composite_expression() -> CompositeExpression {
    let type_ = CompositeExpression::TYPE_AND;
    let expressions = [stub()];

    CompositeExpression::new(type_, expressions).unwrap()
}

#[test]
fn testGetExpressionList() {
    let composite_expression = create_composite_expression();
    let expected_expression_list = vec![stub()];
    let actual_expression_list = composite_expression.get_expression_list();

    assert_eq!(expected_expression_list, actual_expression_list);
}

/// A visitor that counts `walkCompositeExpression()` calls.
#[derive(Default)]
struct Visitor {
    calls: usize,
}

impl ExpressionVisitor for Visitor {
    type Output = ();

    fn walk_comparison(&mut self, _: &Comparison) {}

    fn walk_value(&mut self, _: &Value) {}

    fn walk_composite_expression(&mut self, _: &CompositeExpression) {
        self.calls += 1;
    }
}

#[test]
fn testVisitor() {
    let composite_expression = create_composite_expression();

    let mut visitor = Visitor::default();

    composite_expression.visit(&mut visitor);
    assert_eq!(1, visitor.calls);
}
