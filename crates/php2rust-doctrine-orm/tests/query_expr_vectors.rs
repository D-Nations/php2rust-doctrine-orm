//! The expressions recorded from PHP 8.4.27 and doctrine/orm 3.7.5's
//! `Expr` classes (`oracle/recorders/orm_query_expr.php`,
//! `tests/data/query_expr.json`): each spec is built with the Rust API and
//! its `(string)` (or the exception, without PHP's ", called in ... on
//! line N" suffix) and the deprecations it triggered compared.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::vectors::{self, Outcome};
use php_value::{PhpObject, Thrown, Value};
use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::query::expr::{
    self, Andx, Comparison, Composite, From, Func, GroupBy, Join, Literal, Math, ORDER_DEPRECATION,
    Order, OrderBy, Orx, Select,
};
use php2rust_doctrine_orm::query::{Expr, SortDirection};
use serde_json::Value as Json;

const DATA: &str = include_str!("data/query_expr.json");

fn object(o: impl PhpObject) -> Value {
    Value::Object(Arc::new(o))
}

fn text(value: &Value) -> String {
    String::from_utf8(
        value
            .as_string()
            .expect("a string argument")
            .as_bytes()
            .to_vec(),
    )
    .unwrap()
}

fn opt_text(value: Option<&Value>) -> Option<String> {
    match value {
        None | Some(Value::Null) => None,
        Some(v) => Some(text(v)),
    }
}

fn int(value: &Value) -> i64 {
    match value {
        Value::Int(i) => *i,
        other => panic!("an int argument, got {other:?}"),
    }
}

fn order(value: &Value) -> Order {
    match value {
        Value::Null => Order::Null,
        Value::Object(o) if o.class() == SortDirection::CLASS => {
            let name = o.properties();
            match name.values().next().map(text).as_deref() {
                Some("Descending") => Order::Direction(SortDirection::Descending),
                _ => Order::Direction(SortDirection::Ascending),
            }
        }
        other => Order::Name(text(other)),
    }
}

/// A `SortDirection` case as a value (`["dir", case]`).
fn direction(case: &str) -> Value {
    object(php_value::object::EnumCase::new(
        SortDirection::CLASS,
        case,
        None,
    ))
}

#[allow(clippy::too_many_lines, reason = "one arm per Expr method")]
fn build(spec: &Json) -> Result<Value, Thrown> {
    let kind = spec[0].as_str().unwrap();
    if kind == "v" {
        return Ok(vectors::decode(&spec[1]).unwrap());
    }
    if kind == "dir" {
        return Ok(direction(spec[1].as_str().unwrap()));
    }
    let (head, args) = (&spec[1], spec[2].as_array().unwrap());
    if kind == "add" || kind == "orderAdd" {
        let target = build(head)?;
        let o = target.as_object().unwrap().as_ref();
        if kind == "orderAdd" {
            let built: Vec<Value> = args.iter().map(build).collect::<Result<_, _>>()?;
            let order_by = o.downcast_ref::<OrderBy>().unwrap();
            order_by.add(&text(&built[0]), built.get(1).map(order));
            return Ok(target);
        }
        // Each argument is built just before it is added, as in PHP.
        for arg in args {
            let arg = build(arg)?;
            if let Some(e) = o.downcast_ref::<Andx>() {
                e.add(arg)?;
            } else if let Some(e) = o.downcast_ref::<Orx>() {
                e.add(arg)?;
            } else if let Some(e) = o.downcast_ref::<Select>() {
                e.add(arg)?;
            } else if let Some(e) = o.downcast_ref::<GroupBy>() {
                e.add(arg)?;
            } else {
                panic!("add() on {}", o.class());
            }
        }
        return Ok(target);
    }
    let a: Vec<Value> = args.iter().map(build).collect::<Result<_, _>>()?;
    let arg = |i: usize| a.get(i).cloned().unwrap_or_default();
    let name = head.as_str().unwrap();
    let e = Expr::new();
    if kind == "new" {
        let args_or_empty = || {
            a.first()
                .cloned()
                .unwrap_or(Value::Array(php_value::PhpArray::new()))
        };
        return Ok(match name {
            "Func" => object(Func::new(text(&a[0]), arg(1))),
            "Andx" => object(Andx::new(args_or_empty())?),
            "Orx" => object(Orx::new(args_or_empty())?),
            "Select" => object(Select::new(args_or_empty())?),
            "GroupBy" => object(GroupBy::new(args_or_empty())?),
            "Literal" => object(Literal::new(args_or_empty())?),
            "Composite" => object(Composite::new(args_or_empty())?),
            "Math" => object(Math::new(arg(0), text(&a[1]), arg(2))),
            "Comparison" => object(Comparison::new(arg(0), text(&a[1]), arg(2))),
            "From" => object(From::new(
                text(&a[0]),
                text(&a[1]),
                opt_text(a.get(2)).as_deref(),
            )),
            "Join" => object(Join::new(
                text(&a[0]),
                text(&a[1]),
                opt_text(a.get(2)).as_deref(),
                opt_text(a.get(3)).as_deref(),
                arg(4),
                opt_text(a.get(5)).as_deref(),
            )),
            "OrderBy" => object(OrderBy::new(
                opt_text(a.first()).as_deref(),
                a.get(1)
                    .map_or(Order::Direction(SortDirection::Ascending), order),
            )),
            other => panic!("new {other}"),
        });
    }
    Ok(match name {
        "eq" => e.eq(arg(0), arg(1)).into(),
        "neq" => e.neq(arg(0), arg(1)).into(),
        "lt" => e.lt(arg(0), arg(1)).into(),
        "lte" => e.lte(arg(0), arg(1)).into(),
        "gt" => e.gt(arg(0), arg(1)).into(),
        "gte" => e.gte(arg(0), arg(1)).into(),
        "avg" => e.avg(arg(0)).into(),
        "max" => e.max(arg(0)).into(),
        "min" => e.min(arg(0)).into(),
        "count" => e.count(arg(0)).into(),
        "exists" => e.exists(arg(0)).into(),
        "all" => e.all(arg(0)).into(),
        "some" => e.some(arg(0)).into(),
        "any" => e.any(arg(0)).into(),
        "not" => e.not(arg(0)).into(),
        "abs" => e.abs(arg(0)).into(),
        "sqrt" => e.sqrt(arg(0)).into(),
        "lower" => e.lower(arg(0)).into(),
        "upper" => e.upper(arg(0)).into(),
        "length" => e.length(arg(0)).into(),
        "trim" => e.trim(arg(0)).into(),
        "sum" => e.sum(arg(0), arg(1)).into(),
        "diff" => e.diff(arg(0), arg(1)).into(),
        "prod" => e.prod(arg(0), arg(1)).into(),
        "quot" => e.quot(arg(0), arg(1)).into(),
        "mod" => e.r#mod(arg(0), arg(1)).into(),
        "concat" => e.concat(a.clone()).into(),
        "countDistinct" => Value::String(e.count_distinct(&a)?),
        "substring" => e.substring(arg(0), int(&a[1]), a.get(2).map(int)).into(),
        "literal" => e.literal(arg(0))?.into(),
        "in" => e.r#in(&text(&a[0]), arg(1))?.into(),
        "notIn" => e.not_in(&text(&a[0]), arg(1))?.into(),
        "between" => Value::String(e.between(arg(0), arg(1), arg(2))?),
        "isNull" => Value::from(e.is_null(&text(&a[0]))),
        "isNotNull" => Value::from(e.is_not_null(&text(&a[0]))),
        "like" => e.like(&text(&a[0]), arg(1)).into(),
        "notLike" => e.not_like(&text(&a[0]), arg(1)).into(),
        "isMemberOf" => e.is_member_of(&text(&a[0]), &text(&a[1])).into(),
        "isInstanceOf" => e.is_instance_of(&text(&a[0]), &text(&a[1])).into(),
        "andX" => e.and_x(a.clone())?.into(),
        "orX" => e.or_x(a.clone())?.into(),
        "asc" => e.asc(&text(&a[0])).into(),
        "desc" => e.desc(&text(&a[0])).into(),
        other => panic!("Expr::{other}()"),
    })
}

#[test]
fn recorded_expressions() {
    let cases: Vec<Json> = serde_json::from_str(DATA).unwrap();
    assert_eq!(1463, cases.len());
    deprecations::enable_tracking_deprecations();
    let mut failures = Vec::new();
    for case in &cases {
        let before = deprecations::count(ORDER_DEPRECATION);
        let actual = build(&case["spec"]).and_then(|value| expr::to_string(&value));
        let triggered = deprecations::count(ORDER_DEPRECATION) - before;
        let actual = match actual {
            Ok(s) => Outcome::Value(Value::String(s)),
            Err(thrown) => Outcome::Throws {
                class: thrown.class,
                message: String::from_utf8_lossy(thrown.message.as_bytes()).into_owned(),
            },
        };
        let expected = match vectors::outcome(&case["result"]).unwrap() {
            Outcome::Throws { class, message } => Outcome::Throws {
                class,
                message: message
                    .split_once(", called in ")
                    .map_or(message.clone(), |(m, _)| m.to_owned()),
            },
            value @ Outcome::Value(_) => value,
        };
        if actual != expected || Some(triggered) != case["deprecations"].as_u64() {
            failures.push(format!(
                "{}\n  expected {expected:?} ({} deprecations)\n  actual   {actual:?} ({triggered})",
                case["spec"], case["deprecations"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
