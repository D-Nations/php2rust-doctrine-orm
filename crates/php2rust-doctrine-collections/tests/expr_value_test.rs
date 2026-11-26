//! Port of tests/Expr/ValueTest.php at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d. The mocked visitor counts its
//! calls.
//!
//! Upstream: doctrine/collections tests/Expr/ValueTest.php

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

use php_value::Value as PhpValue;
use php2rust_doctrine_collections::expr::{
    Comparison, CompositeExpression, ExpressionVisitor, Value,
};

#[test]
fn testGetter() {
    let value = PhpValue::from("foo");
    let value_expression = Value::new(value.clone());

    let actual_value = value_expression.get_value();

    assert_eq!(&value, actual_value);
}

/// A visitor that counts `walkValue()` calls.
#[derive(Default)]
struct Visitor {
    calls: usize,
}

impl ExpressionVisitor for Visitor {
    type Output = ();

    fn walk_comparison(&mut self, _: &Comparison) {}

    fn walk_value(&mut self, _: &Value) {
        self.calls += 1;
    }

    fn walk_composite_expression(&mut self, _: &CompositeExpression) {}
}

#[test]
fn testVisitor() {
    let mut visitor = Visitor::default();

    let value = "foo";
    let value_expression = Value::new(value);

    value_expression.visit(&mut visitor);
    assert_eq!(1, visitor.calls);
}
