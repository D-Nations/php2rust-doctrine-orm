//! Port of tests/ClosureExpressionVisitorTest.php at doctrine/collections
//! 7c2f25ed928553a3de389ccb99aa46c0eedb2d4d.
//!
//! Upstream: doctrine/collections tests/ClosureExpressionVisitorTest.php

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

mod common;

use common::{
    TestObject, TestObjectPrivatePropertyOnly, TestObjectPropertyHook,
    TestObjectWithPrivatePropertyInParentClass, assert_equals, assert_thrown, bar_of,
    expect_deprecation, foo_of, usort,
};
use php_value::object::{ArrayIterator, StdClass};
use php_value::{PhpArray, Value, php_array};
use php2rust_doctrine_collections::ExpressionBuilder;
use php2rust_doctrine_collections::expr::{
    ClosureExpressionVisitor, Comparison, CompositeExpression,
};
use std::sync::Arc;

const LINK_486: &str = "https://github.com/doctrine/collections/pull/486";

/// `setUp()`.
fn set_up() -> (ClosureExpressionVisitor, ExpressionBuilder) {
    (
        ClosureExpressionVisitor::new(true),
        ExpressionBuilder::new(),
    )
}

#[test]
fn testEmbeddedObjectComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.eq("foo.foo", 1)).unwrap();
    assert!(closure(&TestObject::object(TestObject::object(1))).unwrap());
    assert!(!closure(&TestObject::object(TestObject::object(2))).unwrap());
}

#[test]
fn testGetObjectFieldValueLegacy() {
    expect_deprecation(LINK_486, || {
        let object = TestObject::object(Value::Null);

        ClosureExpressionVisitor::get_object_field_value_legacy(&object, "foo", true).unwrap();
    });
}

#[test]
fn testGetEmbeddedObjectFieldValueAccessingRawValue() {
    let object = TestObject::object(TestObjectPrivatePropertyOnly::object(42));

    assert_eq!(
        Value::Int(42),
        ClosureExpressionVisitor::get_object_field_value(&object, "foo.fooBar").unwrap()
    );
}

#[test]
fn testGetObjectFieldValueAccessingRawValueBypassingPropertyHook() {
    let object = TestObjectPropertyHook::new();
    object.set_foo_bar(42);

    assert_eq!(
        Value::Int(42),
        ClosureExpressionVisitor::get_object_field_value(&Value::Object(object), "fooBar").unwrap()
    );
}

#[test]
fn testGetObjectFieldValueAccessingRawValue() {
    let object = TestObjectPrivatePropertyOnly::object(42);

    assert_eq!(
        Value::Int(42),
        ClosureExpressionVisitor::get_object_field_value(&object, "fooBar").unwrap()
    );
}

#[test]
fn testGetObjectFieldValueFindingParentClassAccessingRawValue() {
    let object = TestObjectWithPrivatePropertyInParentClass::object(42);

    assert_eq!(
        Value::Int(42),
        ClosureExpressionVisitor::get_object_field_value(&object, "fooBar").unwrap()
    );
}

#[test]
fn testGetObjectFieldValueNonexistentFieldAccessingRawValue() {
    let object = StdClass::object(PhpArray::new());

    assert_thrown(
        ClosureExpressionVisitor::get_object_field_value(&object, "fooBar"),
        "RuntimeException",
        None,
    );
}

#[test]
fn testWalkEqualsComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.eq("foo", 1)).unwrap();

    assert!(closure(&TestObject::object(1)).unwrap());
    assert!(!closure(&TestObject::object(2)).unwrap());
}

#[test]
fn testWalkNotEqualsComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.neq("foo", 1)).unwrap();

    assert!(!closure(&TestObject::object(1)).unwrap());
    assert!(closure(&TestObject::object(2)).unwrap());
}

#[test]
fn testWalkLessThanComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.lt("foo", 1)).unwrap();

    assert!(!closure(&TestObject::object(1)).unwrap());
    assert!(closure(&TestObject::object(0)).unwrap());
}

#[test]
fn testWalkLessThanEqualsComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.lte("foo", 1)).unwrap();

    assert!(!closure(&TestObject::object(2)).unwrap());
    assert!(closure(&TestObject::object(1)).unwrap());
    assert!(closure(&TestObject::object(0)).unwrap());
}

#[test]
fn testWalkGreaterThanEqualsComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.gte("foo", 1)).unwrap();

    assert!(closure(&TestObject::object(2)).unwrap());
    assert!(closure(&TestObject::object(1)).unwrap());
    assert!(!closure(&TestObject::object(0)).unwrap());
}

#[test]
fn testWalkGreaterThanComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.gt("foo", 1)).unwrap();

    assert!(closure(&TestObject::object(2)).unwrap());
    assert!(!closure(&TestObject::object(1)).unwrap());
    assert!(!closure(&TestObject::object(0)).unwrap());
}

#[test]
fn testWalkInComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.r#in("foo", php_array![1, 2, 3, "04"]))
        .unwrap();

    assert!(closure(&TestObject::object(2)).unwrap());
    assert!(closure(&TestObject::object(1)).unwrap());
    assert!(!closure(&TestObject::object(0)).unwrap());
    assert!(!closure(&TestObject::object(4)).unwrap());
    assert!(closure(&TestObject::object("04")).unwrap());
}

#[test]
fn testWalkInComparisonObjects() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.r#in(
            "foo",
            php_array![
                TestObject::object(1),
                TestObject::object(2),
                TestObject::object(4)
            ],
        ))
        .unwrap();

    assert!(closure(&TestObject::object(TestObject::object(2))).unwrap());
    assert!(closure(&TestObject::object(TestObject::object(1))).unwrap());
    assert!(!closure(&TestObject::object(TestObject::object(0))).unwrap());
    assert!(closure(&TestObject::object(TestObject::object(4))).unwrap());
    assert!(!closure(&TestObject::object(TestObject::object("baz"))).unwrap());
}

#[test]
fn testWalkNotInComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.not_in("foo", php_array![1, 2, 3, "04"]))
        .unwrap();

    assert!(!closure(&TestObject::object(1)).unwrap());
    assert!(!closure(&TestObject::object(2)).unwrap());
    assert!(closure(&TestObject::object(0)).unwrap());
    assert!(closure(&TestObject::object(4)).unwrap());
    assert!(!closure(&TestObject::object("04")).unwrap());
}

#[test]
fn testWalkNotInComparisonObjects() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.not_in(
            "foo",
            php_array![
                TestObject::object(1),
                TestObject::object(2),
                TestObject::object(4)
            ],
        ))
        .unwrap();

    assert!(!closure(&TestObject::object(TestObject::object(1))).unwrap());
    assert!(!closure(&TestObject::object(TestObject::object(2))).unwrap());
    assert!(closure(&TestObject::object(TestObject::object(0))).unwrap());
    assert!(!closure(&TestObject::object(TestObject::object(4))).unwrap());
    assert!(closure(&TestObject::object(TestObject::object("baz"))).unwrap());
}

#[test]
fn testWalkContainsComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.contains("foo", "hello"))
        .unwrap();

    assert!(closure(&TestObject::object("hello world")).unwrap());
    assert!(!closure(&TestObject::object("world")).unwrap());
}

#[test]
fn testWalkMemberOfComparisonWithObject() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.member_of("foo", 2))
        .unwrap();

    let iterator = |array: PhpArray| Value::Object(Arc::new(ArrayIterator::new(array)));
    assert!(closure(&TestObject::object(php_array![1, 2, 3])).unwrap());
    assert!(closure(&TestObject::object(php_array![2])).unwrap());
    assert!(closure(&TestObject::object(iterator(php_array![2]))).unwrap());
    assert!(!closure(&TestObject::object(php_array![1, 3, 5])).unwrap());
    assert!(!closure(&TestObject::object(php_array![1, "02"])).unwrap());
    assert!(!closure(&TestObject::object(iterator(php_array![4]))).unwrap());
}

#[test]
fn testWalkStartsWithComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.starts_with("foo", "hello"))
        .unwrap();

    assert!(closure(&TestObject::object("hello world")).unwrap());
    assert!(!closure(&TestObject::object("world")).unwrap());
}

#[test]
fn testWalkEndsWithComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_comparison(&builder.ends_with("foo", "world"))
        .unwrap();

    assert!(closure(&TestObject::object("hello world")).unwrap());
    assert!(!closure(&TestObject::object("hello")).unwrap());
}

#[test]
fn testWalkUnknownOperatorComparisonThrowException() {
    let (visitor, _) = set_up();
    assert_thrown(
        visitor
            .walk_comparison(&Comparison::new("foo", "unknown", 2))
            .and_then(|closure| closure(&TestObject::object(2))),
        "RuntimeException",
        Some("Unknown comparison operator: unknown"),
    );
}

#[test]
fn testWalkAndCompositeExpression() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_composite_expression(
            &builder
                .and_x([builder.eq("foo", 1).into(), builder.eq("bar", 1).into()])
                .unwrap(),
        )
        .unwrap();

    assert!(closure(&TestObject::object2(1, 1)).unwrap());
    assert!(!closure(&TestObject::object2(1, 0)).unwrap());
    assert!(!closure(&TestObject::object2(0, 1)).unwrap());
    assert!(!closure(&TestObject::object2(0, 0)).unwrap());
}

#[test]
fn testWalkOrCompositeExpression() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_composite_expression(
            &builder
                .or_x([builder.eq("foo", 1).into(), builder.eq("bar", 1).into()])
                .unwrap(),
        )
        .unwrap();

    assert!(closure(&TestObject::object2(1, 1)).unwrap());
    assert!(closure(&TestObject::object2(1, 0)).unwrap());
    assert!(closure(&TestObject::object2(0, 1)).unwrap());
    assert!(!closure(&TestObject::object2(0, 0)).unwrap());
}

#[test]
fn testWalkOrAndCompositeExpression() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_composite_expression(
            &builder
                .or_x([
                    builder
                        .and_x([builder.eq("foo", 1).into(), builder.eq("bar", 1).into()])
                        .unwrap()
                        .into(),
                    builder
                        .and_x([builder.eq("foo", 2).into(), builder.eq("bar", 2).into()])
                        .unwrap()
                        .into(),
                ])
                .unwrap(),
        )
        .unwrap();

    assert!(closure(&TestObject::object2(1, 1)).unwrap());
    assert!(closure(&TestObject::object2(2, 2)).unwrap());
    assert!(!closure(&TestObject::object2(1, 2)).unwrap());
    assert!(!closure(&TestObject::object2(2, 1)).unwrap());
    assert!(!closure(&TestObject::object2(0, 0)).unwrap());
}

#[test]
fn testWalkAndOrCompositeExpression() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_composite_expression(
            &builder
                .and_x([
                    builder
                        .or_x([builder.eq("foo", 1).into(), builder.eq("foo", 2).into()])
                        .unwrap()
                        .into(),
                    builder
                        .or_x([builder.eq("bar", 3).into(), builder.eq("bar", 4).into()])
                        .unwrap()
                        .into(),
                ])
                .unwrap(),
        )
        .unwrap();

    assert!(closure(&TestObject::object2(1, 3)).unwrap());
    assert!(closure(&TestObject::object2(1, 4)).unwrap());
    assert!(closure(&TestObject::object2(2, 3)).unwrap());
    assert!(closure(&TestObject::object2(2, 4)).unwrap());
    assert!(!closure(&TestObject::object2(1, 0)).unwrap());
    assert!(!closure(&TestObject::object2(2, 0)).unwrap());
    assert!(!closure(&TestObject::object2(0, 3)).unwrap());
    assert!(!closure(&TestObject::object2(0, 4)).unwrap());
}

#[test]
fn testWalkNotCompositeExpression() {
    let (visitor, builder) = set_up();
    let closure = visitor
        .walk_composite_expression(&builder.not(builder.eq("foo", 1)).unwrap())
        .unwrap();

    assert!(!closure(&TestObject::object(1)).unwrap());
    assert!(closure(&TestObject::object(0)).unwrap());
}

#[test]
fn testWalkUnknownCompositeExpressionThrowException() {
    let (visitor, _) = set_up();
    assert_thrown(
        visitor
            .walk_composite_expression(&CompositeExpression::new("Unknown", []).unwrap())
            .and_then(|closure| closure(&TestObject::object(Value::Null))),
        "RuntimeException",
        Some("Unknown composite Unknown"),
    );
}

#[test]
fn testSortByFieldLegacy() {
    expect_deprecation(LINK_486, || {
        let mut objects = [TestObject::object("b"), TestObject::object("a")];
        let sort = ClosureExpressionVisitor::sort_by_field_legacy("foo", 1, None, true);

        usort(&mut objects, &*sort).unwrap();
    });
}

#[test]
fn testSortByFieldAscending() {
    let mut objects = [
        TestObject::object("b"),
        TestObject::object("a"),
        TestObject::object("c"),
    ];
    let sort = ClosureExpressionVisitor::sort_by_field("foo", 1, None);

    usort(&mut objects, &*sort).unwrap();

    assert_equals(&Value::from("a"), &foo_of(&objects[0]));
    assert_equals(&Value::from("b"), &foo_of(&objects[1]));
    assert_equals(&Value::from("c"), &foo_of(&objects[2]));
}

#[test]
fn testSortByFieldDescending() {
    let mut objects = [
        TestObject::object("b"),
        TestObject::object("a"),
        TestObject::object("c"),
    ];
    let sort = ClosureExpressionVisitor::sort_by_field("foo", -1, None);

    usort(&mut objects, &*sort).unwrap();

    assert_equals(&Value::from("c"), &foo_of(&objects[0]));
    assert_equals(&Value::from("b"), &foo_of(&objects[1]));
    assert_equals(&Value::from("a"), &foo_of(&objects[2]));
}

#[test]
fn testSortByFieldKeepOrderWhenSameValue() {
    let first_element = TestObject::object("a");
    let second_element = TestObject::object("a");

    let mut objects = [first_element.clone(), second_element.clone()];
    let sort = ClosureExpressionVisitor::sort_by_field("foo", 0, None);

    usort(&mut objects, &*sort).unwrap();

    assert_eq!([first_element, second_element], objects);
}

#[test]
fn testSortDelegate() {
    let mut objects = [
        TestObject::object2("a", "c"),
        TestObject::object2("a", "b"),
        TestObject::object2("a", "a"),
    ];
    let sort = ClosureExpressionVisitor::sort_by_field("bar", 1, None);
    let sort = ClosureExpressionVisitor::sort_by_field("foo", 1, Some(sort));

    usort(&mut objects, &*sort).unwrap();

    assert_equals(&Value::from("a"), &bar_of(&objects[0]));
    assert_equals(&Value::from("b"), &bar_of(&objects[1]));
    assert_equals(&Value::from("c"), &bar_of(&objects[2]));
}

#[test]
fn testArrayComparison() {
    let (visitor, builder) = set_up();
    let closure = visitor.walk_comparison(&builder.eq("foo", 42)).unwrap();

    assert!(closure(&Value::Array(php_array!["foo" => 42])).unwrap());
}
