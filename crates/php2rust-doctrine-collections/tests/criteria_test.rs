//! Port of tests/CriteriaTest.php at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d. `orderings()` is
//! deprecated (`#[Deprecated]`), as the tests that call it are marked
//! `#[IgnoreDeprecations]`.
//!
//! Upstream: doctrine/collections tests/CriteriaTest.php

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
#![allow(deprecated)]

use php2rust_doctrine_collections::deprecations;
use php2rust_doctrine_collections::expr::{Comparison, CompositeExpression, Expression};
use php2rust_doctrine_collections::{Criteria, Direction, ExpressionBuilder, Order, SortDirection};

const LINK_486: &str = "https://github.com/doctrine/collections/pull/486";

/// `$this->expectDeprecationWithIdentifier($link)` around `f`.
fn expect_deprecation<T>(link: &str, f: impl FnOnce() -> T) -> T {
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(link);
    let result = f();
    assert!(
        deprecations::count(link) > before,
        "Expected deprecation with identifier \"{link}\" was not triggered"
    );
    deprecations::disable();
    result
}

#[test]
fn testCreateLegacy() {
    let criteria = expect_deprecation(LINK_486, || Criteria::create_legacy(true));

    let _: Criteria = criteria;
}

#[test]
fn testConstructorLegacy() {
    expect_deprecation(LINK_486, || {
        Criteria::new_legacy(
            Some(Comparison::new("test", "=", "test").into()),
            None,
            0,
            None,
            true,
        )
    });
}

#[test]
fn testCreate() {
    let criteria = Criteria::create();

    let _: Criteria = criteria;
}

#[test]
fn testConstructor() {
    let expr: Expression = Comparison::new("field", "=", "value").into();
    let criteria = Criteria::new(
        Some(expr.clone()),
        Some(vec![("foo".to_owned(), Order::Ascending.into())]),
        10,
        Some(20),
    );

    assert_eq!(Some(&expr), criteria.get_where_expression());
    assert_eq!(
        vec![("foo".to_owned(), Order::Ascending)],
        criteria.orderings()
    );
    assert_eq!(Some(10), criteria.get_first_result());
    assert_eq!(Some(20), criteria.get_max_results());
}

#[test]
fn testWhere() {
    let expr = Comparison::new("field", "=", "value");
    let mut criteria = Criteria::create();

    criteria.r#where(expr.clone());

    assert_eq!(
        Some(&Expression::from(expr)),
        criteria.get_where_expression()
    );
}

#[test]
fn testAndWhere() {
    let expr = Comparison::new("field", "=", "value");
    let mut criteria = Criteria::create();

    criteria.r#where(expr);
    let expr = criteria.get_where_expression().unwrap().clone();
    criteria.and_where(expr.clone()).unwrap();

    let Some(Expression::Composite(where_)) = criteria.get_where_expression() else {
        panic!("Failed asserting that the expression is a CompositeExpression");
    };

    assert_eq!(CompositeExpression::TYPE_AND, where_.get_type());
    assert_eq!(&[expr.clone(), expr], where_.get_expression_list());
}

#[test]
fn testAndWhereWithoutWhere() {
    let expr = Comparison::new("field", "=", "value");
    let mut criteria = Criteria::create();

    criteria.and_where(expr.clone()).unwrap();

    assert_eq!(
        Some(&Expression::from(expr)),
        criteria.get_where_expression()
    );
}

#[test]
fn testOrWhere() {
    let expr = Comparison::new("field", "=", "value");
    let mut criteria = Criteria::create();

    criteria.r#where(expr);
    let expr = criteria.get_where_expression().unwrap().clone();
    criteria.or_where(expr.clone()).unwrap();

    let Some(Expression::Composite(where_)) = criteria.get_where_expression() else {
        panic!("Failed asserting that the expression is a CompositeExpression");
    };

    assert_eq!(CompositeExpression::TYPE_OR, where_.get_type());
    assert_eq!(&[expr.clone(), expr], where_.get_expression_list());
}

#[test]
fn testOrWhereWithoutWhere() {
    let expr = Comparison::new("field", "=", "value");
    let mut criteria = Criteria::create();

    criteria.or_where(expr.clone()).unwrap();

    assert_eq!(
        Some(&Expression::from(expr)),
        criteria.get_where_expression()
    );
}

#[test]
fn testOrderings() {
    let mut criteria = Criteria::create();
    criteria.order_by([("foo", SortDirection::Ascending)]);

    assert_eq!(
        vec![("foo".to_owned(), Order::Ascending)],
        criteria.orderings()
    );
}

#[test]
fn testExpr() {
    let _: ExpressionBuilder = Criteria::expr();
}

#[test]
fn testGetOrderings() {
    let mut criteria = Criteria::create();
    criteria.order_by([
        ("foo", Direction::from(SortDirection::Ascending)),
        ("bar", Order::Descending.into()),
    ]);

    assert_eq!(
        vec![
            ("foo".to_owned(), SortDirection::Ascending),
            ("bar".to_owned(), SortDirection::Descending),
        ],
        criteria.get_orderings()
    );
}

#[test]
fn testOrderingsWithMixedOrderTypes() {
    let mut criteria = Criteria::create();
    criteria.order_by([
        ("foo", Direction::from(SortDirection::Ascending)),
        ("bar", Order::Descending.into()),
        ("baz", SortDirection::Descending.into()),
    ]);

    let orderings = criteria.orderings();

    assert_eq!(3, orderings.len());
    assert_eq!(("foo".to_owned(), Order::Ascending), orderings[0]);
    assert_eq!(("bar".to_owned(), Order::Descending), orderings[1]);
    assert_eq!(("baz".to_owned(), Order::Descending), orderings[2]);
}

#[test]
fn testConstructorWithMixedOrderTypes() {
    let expr: Expression = Comparison::new("field", "=", "value").into();
    let criteria = Criteria::new(
        Some(expr.clone()),
        Some(vec![
            ("foo".to_owned(), Order::Ascending.into()),
            ("bar".to_owned(), SortDirection::Descending.into()),
        ]),
        5,
        Some(15),
    );

    assert_eq!(Some(&expr), criteria.get_where_expression());

    let get_orderings = criteria.get_orderings();
    assert_eq!(
        ("foo".to_owned(), SortDirection::Ascending),
        get_orderings[0]
    );
    assert_eq!(
        ("bar".to_owned(), SortDirection::Descending),
        get_orderings[1]
    );

    let orderings = criteria.orderings();
    assert_eq!(("foo".to_owned(), Order::Ascending), orderings[0]);
    assert_eq!(("bar".to_owned(), Order::Descending), orderings[1]);

    assert_eq!(Some(5), criteria.get_first_result());
    assert_eq!(Some(15), criteria.get_max_results());
}
