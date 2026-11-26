//! ExprTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! Names, providers and assertions are PHPUnit's; a provider's rows run in
//! one test, labelled. `(string) $expr` is `try_to_string()`;
//! `assertEquals()` with a string compares the string. The generator rows
//! of the iterable providers are a `\Generator` stand-in that `foreach`
//! traverses (`PhpObject::traverse()`). `VerifyDeprecations` is
//! `crate::deprecations`. The five subquery tests (`exists()`, `all()`,
//! `some()`, `any()`, `not()` of a `QueryBuilder`) pass the builder as
//! `QueryBuilder::to_value()`, a `Stringable` object whose string is the
//! builder's DQL, on the test entity manager (`tests/common/dql.rs`).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Query/ExprTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(non_snake_case)]
#![allow(clippy::arc_with_non_send_sync, reason = "the test entity manager")]

#[path = "common/dql.rs"]
mod dql;
#[path = "common/models.rs"]
mod models;

use php_value::object::StdClass;
use php_value::{PhpArray, PhpObject, PhpString, Value};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::query::expr::{
    Andx, Comparison, From, Func, GroupBy, Join, Literal, Math, ORDER_DEPRECATION, OrderBy, Orx,
    Select,
};
use php2rust_doctrine_orm::query::{Expr, SortDirection};

fn set_up() -> Expr {
    Expr::new()
}

fn s(value: Result<PhpString, php_value::Thrown>) -> String {
    String::from_utf8(value.unwrap().as_bytes().to_vec()).unwrap()
}

fn v(s: &str) -> Value {
    Value::from(s)
}

/// `$gen()`: a `\Generator` that yields the values with keys 0, 1, ...
#[derive(Debug)]
struct Generator(Vec<Value>);

impl PhpObject for Generator {
    fn class(&self) -> &'static str {
        "Generator"
    }

    fn properties(&self) -> PhpArray {
        PhpArray::new()
    }

    fn instance_of(&self, class: &str) -> bool {
        matches!(class, "Traversable" | "Iterator")
    }

    fn traverse(&self) -> Option<Result<Vec<(Value, Value)>, php_value::Thrown>> {
        Some(Ok(self
            .0
            .iter()
            .enumerate()
            .map(|(i, value)| (Value::Int(i64::try_from(i).unwrap()), value.clone()))
            .collect()))
    }
}

fn generator(values: Vec<Value>) -> Value {
    Value::Object(std::sync::Arc::new(Generator(values)))
}

/// `$this->entityManager->createQueryBuilder()` with
/// `select('u')->from('User', 'u')->where('u.name = ?1')`, as the value
/// the expression takes.
fn subquery() -> Value {
    let entity_manager: std::sync::Arc<dyn php2rust_doctrine_orm::query::context::QueryContext> =
        std::sync::Arc::new(dql::TestEntityManager::new());
    let mut qb = php2rust_doctrine_orm::QueryBuilder::new(entity_manager);
    qb.select(vec![v("u")])
        .unwrap()
        .from("User", "u", None)
        .unwrap()
        .r#where(vec![v("u.name = ?1")])
        .unwrap();
    qb.to_value().unwrap()
}

#[test]
fn testExistsExpr() {
    let expr = set_up();
    assert_eq!(
        "EXISTS(SELECT u FROM User u WHERE u.name = ?1)",
        s(expr.exists(subquery()).try_to_string())
    );
}

#[test]
fn testAllExpr() {
    let expr = set_up();
    assert_eq!(
        "ALL(SELECT u FROM User u WHERE u.name = ?1)",
        s(expr.all(subquery()).try_to_string())
    );
}

#[test]
fn testSomeExpr() {
    let expr = set_up();
    assert_eq!(
        "SOME(SELECT u FROM User u WHERE u.name = ?1)",
        s(expr.some(subquery()).try_to_string())
    );
}

#[test]
fn testAnyExpr() {
    let expr = set_up();
    assert_eq!(
        "ANY(SELECT u FROM User u WHERE u.name = ?1)",
        s(expr.any(subquery()).try_to_string())
    );
}

#[test]
fn testNotExpr() {
    let expr = set_up();
    assert_eq!(
        "NOT(SELECT u FROM User u WHERE u.name = ?1)",
        s(expr.not(subquery()).try_to_string())
    );
}

#[test]
fn testAvgExpr() {
    let expr = set_up();
    assert_eq!("AVG(u.id)", s(expr.avg("u.id").try_to_string()));
}

#[test]
fn testMaxExpr() {
    let expr = set_up();
    assert_eq!("MAX(u.id)", s(expr.max("u.id").try_to_string()));
}

#[test]
fn testMinExpr() {
    let expr = set_up();
    assert_eq!("MIN(u.id)", s(expr.min("u.id").try_to_string()));
}

#[test]
fn testCountExpr() {
    let expr = set_up();
    assert_eq!("MAX(u.id)", s(expr.max("u.id").try_to_string()));
}

#[test]
fn testCountDistinctExpr() {
    let expr = set_up();
    assert_eq!("COUNT(DISTINCT u.id)", s(expr.count_distinct(&[v("u.id")])));
}

#[test]
fn testCountDistinctExprMulti() {
    let expr = set_up();
    assert_eq!(
        "COUNT(DISTINCT u.id, u.name)",
        s(expr.count_distinct(&[v("u.id"), v("u.name")]))
    );
}

#[test]
fn testAndExpr() {
    let expr = set_up();
    assert_eq!(
        "1 = 1 AND 2 = 2",
        s(expr
            .and_x(vec![
                Value::String(expr.eq(1, 1).try_to_string().unwrap()),
                Value::String(expr.eq(2, 2).try_to_string().unwrap()),
            ])
            .unwrap()
            .try_to_string())
    );
}

#[test]
fn testIntelligentParenthesisPreventionAndExpr() {
    let expr = set_up();
    assert_eq!(
        "1 = 1 AND 2 = 2",
        s(expr
            .and_x(vec![
                expr.or_x(vec![expr.and_x(vec![expr.eq(1, 1).into()]).unwrap().into()])
                    .unwrap()
                    .into(),
                Value::String(expr.eq(2, 2).try_to_string().unwrap()),
            ])
            .unwrap()
            .try_to_string())
    );
}

#[test]
fn testOrExpr() {
    let expr = set_up();
    assert_eq!(
        "1 = 1 OR 2 = 2",
        s(expr
            .or_x(vec![
                Value::String(expr.eq(1, 1).try_to_string().unwrap()),
                Value::String(expr.eq(2, 2).try_to_string().unwrap()),
            ])
            .unwrap()
            .try_to_string())
    );
}

#[test]
fn testAbsExpr() {
    let expr = set_up();
    assert_eq!("ABS(1)", s(expr.abs(1).try_to_string()));
}

#[test]
fn testProdExpr() {
    let expr = set_up();
    assert_eq!("1 * 2", s(expr.prod(1, 2).try_to_string()));
}

#[test]
fn testDiffExpr() {
    let expr = set_up();
    assert_eq!("1 - 2", s(expr.diff(1, 2).try_to_string()));
}

#[test]
fn testSumExpr() {
    let expr = set_up();
    assert_eq!("1 + 2", s(expr.sum(1, 2).try_to_string()));
}

#[test]
fn testQuotientExpr() {
    let expr = set_up();
    assert_eq!("10 / 2", s(expr.quot(10, 2).try_to_string()));
}

#[test]
fn testScopeInArithmeticExpr() {
    let expr = set_up();
    assert_eq!(
        "(100 - 20) / 2",
        s(expr.quot(expr.diff(100, 20), 2).try_to_string())
    );
    assert_eq!(
        "100 - (20 / 2)",
        s(expr.diff(100, expr.quot(20, 2)).try_to_string())
    );
}

#[test]
fn testSquareRootExpr() {
    let expr = set_up();
    assert_eq!("SQRT(1)", s(expr.sqrt(1).try_to_string()));
}

#[test]
fn testEqualExpr() {
    let expr = set_up();
    assert_eq!("1 = 1", s(expr.eq(1, 1).try_to_string()));
}

#[test]
fn testLikeExpr() {
    let expr = set_up();
    assert_eq!(
        "a.description LIKE :description",
        s(expr.like("a.description", ":description").try_to_string())
    );
}

#[test]
fn testNotLikeExpr() {
    let expr = set_up();
    assert_eq!(
        "a.description NOT LIKE :description",
        s(expr
            .not_like("a.description", ":description")
            .try_to_string())
    );
}

#[test]
fn testConcatExpr() {
    let expr = set_up();
    assert_eq!(
        "CONCAT(u.first_name, u.last_name)",
        s(expr
            .concat(vec![v("u.first_name"), v("u.last_name")])
            .try_to_string())
    );
    assert_eq!(
        "CONCAT(u.first_name, u.middle_name, u.last_name)",
        s(expr
            .concat(vec![
                v("u.first_name"),
                v("u.middle_name"),
                v("u.last_name")
            ])
            .try_to_string())
    );
}

#[test]
fn testSubstringExpr() {
    let expr = set_up();
    assert_eq!(
        "SUBSTRING(a.title, 0, 25)",
        s(expr.substring("a.title", 0, Some(25)).try_to_string())
    );
}

#[test]
fn testModExpr() {
    let expr = set_up();
    assert_eq!("MOD(10, 1)", s(expr.r#mod(10, 1).try_to_string()));
}

/// Groups: regression, DDC-612.
#[test]
fn testSubstringExprAcceptsTwoArguments() {
    let expr = set_up();
    assert_eq!(
        "SUBSTRING(a.title, 5)",
        s(expr.substring("a.title", 5, None).try_to_string())
    );
}

#[test]
fn testLowerExpr() {
    let expr = set_up();
    assert_eq!(
        "LOWER(u.first_name)",
        s(expr.lower("u.first_name").try_to_string())
    );
}

#[test]
fn testUpperExpr() {
    let expr = set_up();
    assert_eq!(
        "UPPER(u.first_name)",
        s(expr.upper("u.first_name").try_to_string())
    );
}

#[test]
fn testLengthExpr() {
    let expr = set_up();
    assert_eq!(
        "LENGTH(u.first_name)",
        s(expr.length("u.first_name").try_to_string())
    );
}

#[test]
fn testGreaterThanExpr() {
    let expr = set_up();
    assert_eq!("5 > 2", s(expr.gt(5, 2).try_to_string()));
}

#[test]
fn testLessThanExpr() {
    let expr = set_up();
    assert_eq!("2 < 5", s(expr.lt(2, 5).try_to_string()));
}

#[test]
fn testStringLiteralExpr() {
    let expr = set_up();
    assert_eq!("'word'", s(expr.literal("word").unwrap().try_to_string()));
}

#[test]
fn testNumericLiteralExpr() {
    let expr = set_up();
    // assertEquals(5, '5'): equal loosely.
    assert_eq!("5", s(expr.literal(5).unwrap().try_to_string()));
}

/// Groups: regression, DDC-610.
#[test]
fn testLiteralExprProperlyQuotesStrings() {
    let expr = set_up();
    assert_eq!(
        "'00010001'",
        s(expr.literal("00010001").unwrap().try_to_string())
    );
}

#[test]
fn testGreaterThanOrEqualToExpr() {
    let expr = set_up();
    assert_eq!("5 >= 2", s(expr.gte(5, 2).try_to_string()));
}

#[test]
fn testLessThanOrEqualTo() {
    let expr = set_up();
    assert_eq!("2 <= 5", s(expr.lte(2, 5).try_to_string()));
}

#[test]
fn testBetweenExpr() {
    let expr = set_up();
    assert_eq!("u.id BETWEEN 3 AND 6", s(expr.between("u.id", 3, 6)));
}

#[test]
fn testTrimExpr() {
    let expr = set_up();
    assert_eq!("TRIM(u.id)", s(expr.trim("u.id").try_to_string()));
}

#[test]
fn testIsNullExpr() {
    let expr = set_up();
    assert_eq!("u.id IS NULL", expr.is_null("u.id"));
}

#[test]
fn testIsNotNullExpr() {
    let expr = set_up();
    assert_eq!("u.id IS NOT NULL", expr.is_not_null("u.id"));
}

#[test]
fn testIsInstanceOfExpr() {
    let expr = set_up();
    assert_eq!(
        "u INSTANCE OF Doctrine\\Tests\\Models\\Company\\CompanyEmployee",
        s(expr
            .is_instance_of("u", "Doctrine\\Tests\\Models\\Company\\CompanyEmployee")
            .try_to_string())
    );
}

#[test]
fn testIsMemberOfExpr() {
    let expr = set_up();
    assert_eq!(
        ":groupId MEMBER OF u.groups",
        s(expr.is_member_of(":groupId", "u.groups").try_to_string())
    );
}

fn provide_iterable_value() -> Vec<(&'static str, Value)> {
    let values = || vec![Value::Int(1), Value::Int(2), Value::Int(3)];
    vec![
        ("simple_array", Value::Array(PhpArray::list(values()))),
        ("generator", generator(values())),
    ]
}

fn provide_literal_iterable_value() -> Vec<(&'static str, Value)> {
    let values = || vec![v("foo"), v("bar")];
    vec![
        ("simple_array", Value::Array(PhpArray::list(values()))),
        ("generator", generator(values())),
    ]
}

#[test]
fn testInExpr() {
    for (label, value) in provide_iterable_value() {
        let expr = set_up();
        assert_eq!(
            "u.id IN(1, 2, 3)",
            s(expr.r#in("u.id", value).unwrap().try_to_string()),
            "{label}"
        );
    }
}

#[test]
fn testInLiteralExpr() {
    for (label, value) in provide_literal_iterable_value() {
        let expr = set_up();
        assert_eq!(
            "u.type IN('foo', 'bar')",
            s(expr.r#in("u.type", value).unwrap().try_to_string()),
            "{label}"
        );
    }
}

#[test]
fn testNotInExpr() {
    for (label, value) in provide_iterable_value() {
        let expr = set_up();
        assert_eq!(
            "u.id NOT IN(1, 2, 3)",
            s(expr.not_in("u.id", value).unwrap().try_to_string()),
            "{label}"
        );
    }
}

#[test]
fn testNotInLiteralExpr() {
    for (label, value) in provide_literal_iterable_value() {
        let expr = set_up();
        assert_eq!(
            "u.type NOT IN('foo', 'bar')",
            s(expr.not_in("u.type", value).unwrap().try_to_string()),
            "{label}"
        );
    }
}

#[test]
fn testAndxOrxExpr() {
    let expr = set_up();
    let and_expr = expr.and_x(vec![]).unwrap();
    and_expr.add(expr.eq(1, 1)).unwrap();
    and_expr.add(expr.lt(1, 5)).unwrap();

    let or_expr = expr.or_x(vec![]).unwrap();
    or_expr.add(and_expr).unwrap();
    or_expr.add(expr.eq(1, 1)).unwrap();

    assert_eq!("(1 = 1 AND 1 < 5) OR 1 = 1", s(or_expr.try_to_string()));
}

#[test]
fn testOrxExpr() {
    let expr = set_up();
    let or_expr = expr.or_x(vec![]).unwrap();
    or_expr.add(expr.eq(1, 1)).unwrap();
    or_expr.add(expr.lt(1, 5)).unwrap();

    assert_eq!("1 = 1 OR 1 < 5", s(or_expr.try_to_string()));
}

#[test]
fn testOrderByCountExpr() {
    let expr = set_up();
    let order_expr = expr.desc("u.username");

    assert_eq!(order_expr.count(), 1);
    assert_eq!("u.username DESC", s(order_expr.try_to_string()));
}

#[test]
fn testOrderByOrder() {
    let expr = set_up();
    let order_expr = expr.desc("u.username");
    assert_eq!("u.username DESC", s(order_expr.try_to_string()));
}

#[test]
fn testOrderByAsc() {
    let expr = set_up();
    let order_expr = expr.asc("u.username");
    assert_eq!("u.username ASC", s(order_expr.try_to_string()));
}

#[test]
fn testAddThrowsException() {
    let expr = set_up();
    let or_expr = expr.or_x(vec![]).unwrap();
    let error = or_expr.add(expr.quot(5, 2)).unwrap_err();
    assert_eq!("InvalidArgumentException", error.class);
}

fn provide_invalid_types_for_add() -> Vec<(&'static str, Value)> {
    let mut foo_bar = PhpArray::new();
    foo_bar.insert("foo", v("bar"));
    vec![
        ("integer 1", Value::Int(1)),
        ("object", StdClass::object(foo_bar.clone())),
        ("array", Value::Array(foo_bar)),
    ]
}

#[test]
fn testAddThrowsExceptionOnInvalidType() {
    for (label, arg) in provide_invalid_types_for_add() {
        let expr = set_up();
        let or_expr = expr.or_x(vec![]).unwrap();
        let error = or_expr.add(arg).unwrap_err();
        assert_eq!("InvalidArgumentException", error.class, "{label}");
    }
}

/// Group: DDC-1683.
#[test]
fn testBooleanLiteral() {
    let expr = set_up();
    assert_eq!("true", s(expr.literal(true).unwrap().try_to_string()));
    assert_eq!("false", s(expr.literal(false).unwrap().try_to_string()));
}

/// Group: DDC-1686.
#[test]
fn testExpressionGetter() {
    // Andx
    let andx = Andx::new(Value::Array(PhpArray::list([v("1 = 1"), v("2 = 2")]))).unwrap();
    assert!(values_equal(&[v("1 = 1"), v("2 = 2")], &andx.get_parts()));

    // Comparison
    let comparison = Comparison::new("foo", Comparison::EQ, "bar");
    assert!(values_equal(
        &[v("foo")],
        std::slice::from_ref(comparison.get_left_expr())
    ));
    assert!(values_equal(
        &[v("bar")],
        std::slice::from_ref(comparison.get_right_expr())
    ));
    assert_eq!(Comparison::EQ, comparison.get_operator());

    // From
    let from = From::new("Foo", "f", Some("f.id"));
    assert_eq!("f", from.get_alias());
    assert_eq!("Foo", from.get_from());
    assert_eq!(Some("f.id"), from.get_index_by());

    // Func
    let func = Func::new("MAX", Value::Array(PhpArray::list([v("f.id")])));
    assert_eq!("MAX", func.get_name());
    assert_eq!(&PhpArray::list([v("f.id")]), func.get_arguments());

    // GroupBy
    let group = GroupBy::new(Value::Array(PhpArray::list([v("foo DESC"), v("bar ASC")]))).unwrap();
    assert!(values_equal(
        &[v("foo DESC"), v("bar ASC")],
        &group.get_parts()
    ));

    // Join
    let join = Join::new(
        Join::INNER_JOIN,
        "f.bar",
        Some("b"),
        Some(Join::WITH),
        "b.bar_id = 1",
        Some("b.bar_id"),
    );
    assert_eq!(Join::INNER_JOIN, join.get_join_type());
    assert_eq!(Some(Join::WITH), join.get_condition_type());
    assert!(values_equal(
        &[v("b.bar_id = 1")],
        std::slice::from_ref(join.get_condition())
    ));
    assert_eq!(Some("b.bar_id"), join.get_index_by());
    assert_eq!("f.bar", join.get_join());
    assert_eq!(Some("b"), join.get_alias());

    // Literal
    let literal = Literal::new(Value::Array(PhpArray::list([v("foo")]))).unwrap();
    assert!(values_equal(&[v("foo")], &literal.get_parts()));

    // Math
    let math = Math::new(10, "+", 20);
    assert!(values_equal(
        &[Value::Int(10)],
        std::slice::from_ref(math.get_left_expr())
    ));
    assert!(values_equal(
        &[Value::Int(20)],
        std::slice::from_ref(math.get_right_expr())
    ));
    assert_eq!("+", math.get_operator());

    // OrderBy
    let order = OrderBy::new(Some("foo"), SortDirection::Descending);
    assert_eq!(vec!["foo DESC".to_owned()], order.get_parts());

    // Andx
    let orx = Orx::new(Value::Array(PhpArray::list([v("foo = 1"), v("bar = 2")]))).unwrap();
    assert!(values_equal(
        &[v("foo = 1"), v("bar = 2")],
        &orx.get_parts()
    ));

    // Select
    let select = Select::new(Value::Array(PhpArray::list([v("foo"), v("bar")]))).unwrap();
    assert!(values_equal(&[v("foo"), v("bar")], &select.get_parts()));
}

/// `assertEquals()` of two lists.
fn values_equal(expected: &[Value], actual: &[Value]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(a, b)| php_value::compare::loose_equals(a, b))
}

#[test]
fn testAddEmpty() {
    let expr = set_up();
    let and_expr = expr.and_x(vec![]).unwrap();
    and_expr.add(expr.and_x(vec![]).unwrap()).unwrap();

    assert_eq!(0, and_expr.count());
}

#[test]
fn testAddNull() {
    let expr = set_up();
    let and_expr = expr.and_x(vec![]).unwrap();
    and_expr.add(Value::Null).unwrap();

    assert_eq!(0, and_expr.count());
}

/// `expectDeprecationWithIdentifier()`: tracking on, and the count of
/// the link checked after the test.
fn expect_deprecation_with_identifier(identifier: &str) -> impl FnOnce() + '_ {
    deprecations::enable_tracking_deprecations();
    let before = deprecations::count(identifier);
    move || {
        assert!(
            deprecations::count(identifier) > before,
            "Expected deprecation with identifier \"{identifier}\" was not triggered"
        );
    }
}

/// `#[IgnoreDeprecations]`
#[test]
fn testLegacyOrderByConstruct() {
    let verify = expect_deprecation_with_identifier(ORDER_DEPRECATION);
    let order_expr = OrderBy::new(Some("u.username"), "DESC");
    assert_eq!("u.username DESC", s(order_expr.try_to_string()));
    verify();
}

/// `#[IgnoreDeprecations]`
#[test]
fn testLegacyOrderByAdd() {
    let order_expr = OrderBy::new(Some("u.username"), SortDirection::Descending);
    let verify = expect_deprecation_with_identifier(ORDER_DEPRECATION);
    order_expr.add("u.lastname", Some("DESC".into()));
    assert_eq!(
        "u.username DESC, u.lastname DESC",
        s(order_expr.try_to_string())
    );
    verify();
}
