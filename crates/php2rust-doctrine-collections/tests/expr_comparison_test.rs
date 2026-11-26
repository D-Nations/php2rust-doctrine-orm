//! Port of tests/Expr/ComparisonTest.php at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d. The mocked visitor is a
//! visitor that records its calls.
//!
//! Upstream: doctrine/collections tests/Expr/ComparisonTest.php

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
    Comparison, CompositeExpression, ExpressionVisitor, Value,
};

/// A visitor whose `walkComparison()` is expected once, with
/// `expected`, and returns `returned`.
struct Visitor {
    expected: Comparison,
    returned: &'static str,
    calls: usize,
}

impl ExpressionVisitor for Visitor {
    type Output = Option<&'static str>;

    fn walk_comparison(&mut self, comparison: &Comparison) -> Self::Output {
        assert_eq!(&self.expected, comparison);
        self.calls += 1;
        Some(self.returned)
    }

    fn walk_value(&mut self, _: &Value) -> Self::Output {
        None
    }

    fn walk_composite_expression(&mut self, _: &CompositeExpression) -> Self::Output {
        None
    }
}

#[test]
fn testVisit() {
    let callable_expected = "callable";

    let comparison = Comparison::new("foo", Comparison::EQ, 3);

    let mut visitor = Visitor {
        expected: comparison.clone(),
        returned: callable_expected,
        calls: 0,
    };

    assert_eq!(Some(callable_expected), comparison.visit(&mut visitor));
    assert_eq!(1, visitor.calls);
}
