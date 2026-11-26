//! QueryExpressionVisitorTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The `comparisonData` rows run in one test, labelled. PHPUnit's
//! `assertEquals()` of two expression objects compares their class and
//! properties (`DqlExpression::array_cast()`); of two parameter collections,
//! each parameter's name, value and type.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/QueryExpressionVisitorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]

use php_value::compare::loose_equals;
use php_value::{PhpArray, Value};
use php2rust_doctrine_collections::ExpressionBuilder as CriteriaBuilder;
use php2rust_doctrine_collections::expr::{
    Comparison as CriteriaComparison, ExpressionVisitor, Value as CriteriaValue,
};
use php2rust_doctrine_orm::query::expr::{Andx, Comparison, DqlExpression, Func, Orx};
use php2rust_doctrine_orm::query::{Expr as QueryBuilder, Parameter, QueryExpressionVisitor};

fn set_up() -> QueryExpressionVisitor {
    QueryExpressionVisitor::new(vec!["o".to_owned(), "p".to_owned()])
}

/// The class and `(array)` cast of an expression value (a string is
/// itself).
fn shape(value: &Value) -> (String, Value) {
    let Value::Object(object) = value else {
        return ("string".to_owned(), value.clone());
    };
    let array = if let Some(e) = object.downcast_ref::<Comparison>() {
        e.array_cast()
    } else if let Some(e) = object.downcast_ref::<Func>() {
        e.array_cast()
    } else if let Some(e) = object.downcast_ref::<Andx>() {
        e.array_cast()
    } else if let Some(e) = object.downcast_ref::<Orx>() {
        e.array_cast()
    } else {
        panic!("not an expression: {}", object.class());
    };
    (object.class().to_owned(), Value::Array(array))
}

fn assert_expr_equals(expected: &Value, actual: &Value, label: &str) {
    let (expected_class, expected) = shape(expected);
    let (actual_class, actual) = shape(actual);
    assert_eq!(expected_class, actual_class, "{label}");
    assert!(
        loose_equals(&expected, &actual),
        "{label}: {expected:?} != {actual:?}"
    );
}

fn assert_parameters_equal(expected: &[Parameter], actual: &[Parameter], label: &str) {
    assert_eq!(expected.len(), actual.len(), "{label}");
    for (e, a) in expected.iter().zip(actual) {
        assert_eq!(e.get_name(), a.get_name(), "{label}");
        assert!(loose_equals(e.get_value(), a.get_value()), "{label}");
        assert_eq!(e.get_type(), a.get_type(), "{label}");
        assert_eq!(e.type_was_specified(), a.type_was_specified(), "{label}");
    }
}

fn list(values: &[&str]) -> PhpArray {
    PhpArray::list(values.iter().map(|v| Value::from(*v)))
}

#[allow(
    clippy::type_complexity,
    clippy::too_many_lines,
    reason = "the provider's rows"
)]
fn comparison_data() -> Vec<(&'static str, CriteriaComparison, Value, Option<Parameter>)> {
    let cb = CriteriaBuilder::new();
    let qb = QueryBuilder::new();

    vec![
        (
            "eq",
            cb.eq("field", "value"),
            qb.eq("o.field", ":field").into(),
            Some(Parameter::new("field", "value", None)),
        ),
        (
            "neq",
            cb.neq("field", "value"),
            qb.neq("o.field", ":field").into(),
            Some(Parameter::new("field", "value", None)),
        ),
        (
            "eq null",
            cb.eq("field", Value::Null),
            qb.is_null("o.field").into(),
            None,
        ),
        (
            "neq null",
            cb.neq("field", Value::Null),
            qb.is_not_null("o.field").into(),
            None,
        ),
        (
            "isNull",
            cb.is_null("field"),
            qb.is_null("o.field").into(),
            None,
        ),
        (
            "gt",
            cb.gt("field", "value"),
            qb.gt("o.field", ":field").into(),
            Some(Parameter::new("field", "value", None)),
        ),
        (
            "gte",
            cb.gte("field", "value"),
            qb.gte("o.field", ":field").into(),
            Some(Parameter::new("field", "value", None)),
        ),
        (
            "lt",
            cb.lt("field", "value"),
            qb.lt("o.field", ":field").into(),
            Some(Parameter::new("field", "value", None)),
        ),
        (
            "lte",
            cb.lte("field", "value"),
            qb.lte("o.field", ":field").into(),
            Some(Parameter::new("field", "value", None)),
        ),
        (
            "in",
            cb.r#in("field", list(&["value"])),
            qb.r#in("o.field", ":field").unwrap().into(),
            Some(Parameter::new(
                "field",
                Value::Array(list(&["value"])),
                None,
            )),
        ),
        (
            "notIn",
            cb.not_in("field", list(&["value"])),
            qb.not_in("o.field", ":field").unwrap().into(),
            Some(Parameter::new(
                "field",
                Value::Array(list(&["value"])),
                None,
            )),
        ),
        (
            "contains",
            cb.contains("field", "value"),
            qb.like("o.field", ":field").into(),
            Some(Parameter::new("field", "%value%", None)),
        ),
        (
            "memberOf",
            cb.member_of(":field", "o.field"),
            qb.is_member_of(":field", "o.field").into(),
            None,
        ),
        (
            "startsWith",
            cb.starts_with("field", "value"),
            qb.like("o.field", ":field").into(),
            Some(Parameter::new("field", "value%", None)),
        ),
        (
            "endsWith",
            cb.ends_with("field", "value"),
            qb.like("o.field", ":field").into(),
            Some(Parameter::new("field", "%value", None)),
        ),
        // Test parameter conversion
        (
            "parameter conversion",
            cb.eq("object.field", "value"),
            qb.eq("o.object.field", ":object_field").into(),
            Some(Parameter::new("object_field", "value", None)),
        ),
        // Test alternative rootAlias
        (
            "alternative root alias",
            cb.eq("p.field", "value"),
            qb.eq("p.field", ":p_field").into(),
            Some(Parameter::new("p_field", "value", None)),
        ),
        (
            "alternative root alias with path",
            cb.eq("p.object.field", "value"),
            qb.eq("p.object.field", ":p_object_field").into(),
            Some(Parameter::new("p_object_field", "value", None)),
        ),
    ]
}

#[test]
fn testWalkComparison() {
    for (label, criteria_expr, query_expr, parameter) in comparison_data() {
        let mut visitor = set_up();
        assert_expr_equals(
            &query_expr,
            &visitor.walk_comparison(&criteria_expr).unwrap(),
            label,
        );
        if let Some(parameter) = parameter {
            assert_parameters_equal(&[parameter], &visitor.get_parameters(), label);
        }
    }
}

#[test]
fn testWalkAndCompositeExpression() {
    let mut visitor = set_up();
    let cb = CriteriaBuilder::new();
    let expr = visitor
        .walk_composite_expression(
            &cb.and_x([cb.eq("foo", 1).into(), cb.eq("bar", 1).into()])
                .unwrap(),
        )
        .unwrap();

    let andx = expr
        .as_object()
        .and_then(|o| o.downcast_ref::<Andx>())
        .expect("an Andx");
    assert_eq!(2, andx.get_parts().len());
}

#[test]
fn testWalkOrCompositeExpression() {
    let mut visitor = set_up();
    let cb = CriteriaBuilder::new();
    let expr = visitor
        .walk_composite_expression(
            &cb.or_x([cb.eq("foo", 1).into(), cb.eq("bar", 1).into()])
                .unwrap(),
        )
        .unwrap();

    let orx = expr
        .as_object()
        .and_then(|o| o.downcast_ref::<Orx>())
        .expect("an Orx");
    assert_eq!(2, orx.get_parts().len());
}

#[test]
fn testWalkNotCompositeExpression() {
    let mut visitor = set_up();
    let qb = QueryBuilder::new();
    let cb = CriteriaBuilder::new();

    let expr = visitor
        .walk_composite_expression(&cb.not(cb.eq("foo", 1)).unwrap())
        .unwrap();

    let func = expr
        .as_object()
        .and_then(|o| o.downcast_ref::<Func>())
        .expect("a Func");
    assert_eq!("NOT", func.get_name());
    assert_eq!(1, func.get_arguments().len());
    assert_expr_equals(
        &qb.eq("o.foo", ":foo").into(),
        func.get_arguments().values().next().unwrap(),
        "argument",
    );
    assert_parameters_equal(
        &[Parameter::new("foo", 1, None)],
        &visitor.get_parameters(),
        "parameters",
    );
}

#[test]
fn testWalkValue() {
    let mut visitor = set_up();
    assert!(loose_equals(
        &Value::from("value"),
        &visitor.walk_value(&CriteriaValue::new("value")).unwrap()
    ));
}

#[test]
fn testClearParameters() {
    let mut visitor = set_up();
    visitor
        .get_parameters()
        .push(Parameter::new("field", "value", None));

    visitor.clear_parameters();

    assert_eq!(0, visitor.get_parameters().len());
}
