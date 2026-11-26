//! Port of tests/ExpressionBuilderTest.php at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d. The `assertInstanceOf`
//! checks are the result types.
//!
//! Upstream: doctrine/collections tests/ExpressionBuilderTest.php

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

use php_value::php_array;
use php2rust_doctrine_collections::ExpressionBuilder;
use php2rust_doctrine_collections::expr::{Comparison, CompositeExpression};

/// `setUp()`.
fn set_up() -> ExpressionBuilder {
    ExpressionBuilder::new()
}

#[test]
fn testAndX() {
    let builder = set_up();
    let expr = builder.and_x([builder.eq("a", "b").into()]).unwrap();

    let _: &CompositeExpression = &expr;
    assert_eq!(CompositeExpression::TYPE_AND, expr.get_type());
}

#[test]
fn testOrX() {
    let builder = set_up();
    let expr = builder.or_x([builder.eq("a", "b").into()]).unwrap();

    let _: &CompositeExpression = &expr;
    assert_eq!(CompositeExpression::TYPE_OR, expr.get_type());
}

#[test]
fn testEq() {
    let builder = set_up();
    let expr = builder.eq("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::EQ, expr.get_operator());
}

#[test]
fn testNeq() {
    let builder = set_up();
    let expr = builder.neq("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::NEQ, expr.get_operator());
}

#[test]
fn testLt() {
    let builder = set_up();
    let expr = builder.lt("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::LT, expr.get_operator());
}

#[test]
fn testGt() {
    let builder = set_up();
    let expr = builder.gt("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::GT, expr.get_operator());
}

#[test]
fn testGte() {
    let builder = set_up();
    let expr = builder.gte("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::GTE, expr.get_operator());
}

#[test]
fn testLte() {
    let builder = set_up();
    let expr = builder.lte("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::LTE, expr.get_operator());
}

#[test]
fn testIn() {
    let builder = set_up();
    let expr = builder.r#in("a", php_array!["b"]);

    let _: &Comparison = &expr;
    assert_eq!(Comparison::IN, expr.get_operator());
}

#[test]
fn testNotIn() {
    let builder = set_up();
    let expr = builder.not_in("a", php_array!["b"]);

    let _: &Comparison = &expr;
    assert_eq!(Comparison::NIN, expr.get_operator());
}

#[test]
fn testIsNull() {
    let builder = set_up();
    let expr = builder.is_null("a");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::EQ, expr.get_operator());
}

#[test]
fn testIsNotNull() {
    let builder = set_up();
    let expr = builder.is_not_null("a");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::NEQ, expr.get_operator());
}

#[test]
fn testContains() {
    let builder = set_up();
    let expr = builder.contains("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::CONTAINS, expr.get_operator());
}

#[test]
fn testMemberOf() {
    let builder = set_up();
    let expr = builder.member_of("b", php_array!["a"]);

    let _: &Comparison = &expr;
    assert_eq!(Comparison::MEMBER_OF, expr.get_operator());
}

#[test]
fn testStartsWith() {
    let builder = set_up();
    let expr = builder.starts_with("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::STARTS_WITH, expr.get_operator());
}

#[test]
fn testEndsWith() {
    let builder = set_up();
    let expr = builder.ends_with("a", "b");

    let _: &Comparison = &expr;
    assert_eq!(Comparison::ENDS_WITH, expr.get_operator());
}
