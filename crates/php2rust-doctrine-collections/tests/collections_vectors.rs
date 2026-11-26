//! Replays the answers real PHP 8.4.27 gave with doctrine/collections
//! (`oracle/recorders/doctrine_collections.php`, `scripts/record.sh
//! collections`) for what the upstream tests leave open:
//! `ArrayCollection`'s internal pointer under random call sequences,
//! `ClosureExpressionVisitor`'s comparisons over PHP values, field reads
//! and their errors, composite expressions over arrays, callable
//! criteria, `matching()`'s sorting and slicing, `slice()`, and messages.
//!
//! PHP's `TypeError`s for user functions end with `, called in <file> on
//! line <line>`, which the port leaves out: such a message matches when
//! PHP's starts with the port's and that tail.
//!
//! Upstream: none

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::OnceLock;

use php_value::vectors::{Outcome, decode, outcome};
use php_value::{Key, PhpArray, Thrown, Value};
use php2rust_doctrine_collections::expr::{ClosureExpressionVisitor, Comparison, Expression};
use php2rust_doctrine_collections::{
    ArrayCollection, Criteria, Direction, ExpressionBuilder, Order, SortDirection,
};
use serde_json::Value as Json;

fn data() -> &'static Json {
    static DATA: OnceLock<Json> = OnceLock::new();
    DATA.get_or_init(|| {
        let text = include_str!("data/doctrine_collections.json");
        serde_json::from_str(text).unwrap()
    })
}

fn value(json: &Json) -> Value {
    decode(json).unwrap()
}

fn key(json: &Json) -> Key {
    Key::from_value(&value(json)).unwrap()
}

/// Whether the port's result is what PHP recorded.
fn same(expected: &Outcome, actual: &Result<Value, Thrown>) -> bool {
    match (expected, actual) {
        (Outcome::Value(e), Ok(a)) => e == a,
        (Outcome::Throws { class, message }, Err(thrown)) => {
            let ours = thrown.message.to_string_lossy();
            *class == thrown.class
                && (*message == ours || message.starts_with(&format!("{ours}, called in ")))
        }
        _ => false,
    }
}

fn check(context: &str, expected: &Json, actual: &Result<Value, Thrown>) {
    let expected = outcome(expected).unwrap();
    assert!(
        same(&expected, actual),
        "{context}: PHP gave {expected:?}, the port {actual:?}"
    );
}

/// Replays the call sequences. Where the pointer is past the end and
/// elements are added, whether PHP's pointer then reaches them depends
/// on the array's internal layout (packed or hash, holes), which a
/// `PhpArray` does not have: the port's reaches the first added one. From
/// there until `first()`, `last()` or `clear()` sets the pointer again,
/// the pointer's answers are not compared.
#[test]
fn internal_pointer() {
    let (mut compared, mut skipped, mut differing) = (0, 0, 0);
    for (n, sequence) in data()["pointer"].as_array().unwrap().iter().enumerate() {
        let initial = value(&sequence["initial"]);
        let mut c = ArrayCollection::new(initial.as_array().unwrap().clone());
        let mut pointer_known = true;
        for (i, step) in sequence["steps"].as_array().unwrap().iter().enumerate() {
            let op = step[0].as_str().unwrap();
            let (k, v) = (key(&step[1]), value(&step[2]));
            let past_end = c.key().is_none();
            let adds = match op {
                "set" => !c.contains_key(&k),
                "add" => true,
                _ => false,
            };
            if past_end && adds {
                pointer_known = false;
            }
            if matches!(op, "first" | "last" | "clear") {
                pointer_known = true;
            }
            let result: Result<Value, Thrown> = match op {
                "next" => Ok(c.next()),
                "first" => Ok(c.first()),
                "last" => Ok(c.last()),
                "remove" => Ok(c.remove(&k)),
                "set" => {
                    c.set(k, v);
                    Ok(Value::Null)
                }
                "add" => c.add(v).map(|()| Value::Null),
                "removeElement" => Ok(Value::Bool(c.remove_element(&v))),
                "clear" => {
                    c.clear();
                    Ok(Value::Null)
                }
                "matching" => c.matching(&Criteria::create()).map(|m| {
                    c = m;
                    Value::Null
                }),
                other => panic!("unknown op {other}"),
            };
            let context = format!("sequence {n} step {i} ({op})");
            assert_eq!(
                value(&step[6]),
                Value::Array(c.to_array()),
                "{context}: toArray()"
            );
            if !pointer_known {
                skipped += 1;
                let php_key = value(&step[4]);
                if php_key != c.key().map_or(Value::Null, |k| k.to_value()) {
                    differing += 1;
                }
                continue;
            }
            compared += 1;
            check(&context, &step[3], &result);
            assert_eq!(
                value(&step[4]),
                c.key().map_or(Value::Null, |k| k.to_value()),
                "{context}: key()"
            );
            assert_eq!(value(&step[5]), c.current(), "{context}: current()");
        }
    }
    println!("{compared} compared, {skipped} skipped, of which {differing} differ");
    assert!(
        compared > 5000 && skipped < compared / 2,
        "{compared} compared, {skipped} skipped"
    );
}

#[test]
fn comparisons() {
    let visitor = ClosureExpressionVisitor::default();
    let mut count = 0;
    for comparison in data()["comparisons"].as_array().unwrap() {
        let op = comparison["op"].as_str().unwrap();
        let operand = value(&comparison["value"]);
        let closure = visitor
            .walk_comparison(&Comparison::new("foo", op, operand.clone()))
            .unwrap();
        for answer in comparison["answers"].as_array().unwrap() {
            let field = value(&answer[0]);
            let mut element = PhpArray::new();
            element.insert("foo", field.clone());
            let actual = closure(&Value::Array(element)).map(Value::Bool);
            check(&format!("{field:?} {op} {operand:?}"), &answer[1], &actual);
            count += 1;
        }
    }
    assert!(count > 5000, "{count} comparisons");
}

#[test]
fn field_values() {
    for field in data()["fields"].as_array().unwrap() {
        let object = value(&field[0]);
        let name = field[1].as_str().unwrap();
        let actual = ClosureExpressionVisitor::get_object_field_value(&object, name);
        check(&format!("{object:?} {name}"), &field[2], &actual);
    }
}

#[test]
fn composite_expressions() {
    let builder = ExpressionBuilder::new();
    let visitor = ClosureExpressionVisitor::default();
    for composite in data()["composites"].as_array().unwrap() {
        let type_ = composite[0].as_str().unwrap();
        let eq: Expression = builder.eq("foo", 1).into();
        let expr = match type_ {
            "and" => builder.and_x([eq]),
            "or" => builder.or_x([eq]),
            _ => builder.not(eq),
        }
        .unwrap();
        let closure = visitor.walk_composite_expression(&expr).unwrap();
        let element = value(&composite[1]);
        let actual = closure(&element).map(Value::Bool);
        check(&format!("{type_} {element:?}"), &composite[2], &actual);
    }
}

#[test]
fn callable_criteria() {
    for callable in data()["callables"].as_array().unwrap() {
        let callback = value(&callable[0]);
        let elements = php_value::php_array![
            0,
            1,
            "",
            "a",
            Value::Null,
            PhpArray::new(),
            php_value::php_array![0]
        ];
        let criteria = Criteria::new(
            Some(php2rust_doctrine_collections::expr::Value::new(callback.clone()).into()),
            None,
            0,
            None,
        );
        let actual = ArrayCollection::new(elements)
            .matching(&criteria)
            .map(|c| Value::Array(c.to_array()));
        check(&format!("{callback:?}"), &callable[1], &actual);
    }
}

fn direction(kind: &str, name: &str) -> Direction {
    match (kind, name) {
        ("Order", "Ascending") => Order::Ascending.into(),
        ("Order", _) => Order::Descending.into(),
        (_, "Ascending") => SortDirection::Ascending.into(),
        _ => SortDirection::Descending.into(),
    }
}

/// The pools whose values compare consistently (no two values loosely
/// equal but not identical). In the others (`1` and `'1'`, `null` and
/// `false`), `sortByField()` answers `-1` both ways for such a pair, and
/// the order depends on the sort algorithm's comparisons: PHP's
/// `zend_sort()` is not reproduced, so there only the elements are
/// compared, not their order.
const CONSISTENT: [&str; 4] = ["ints", "floats", "strings", "numeric strings"];

#[test]
fn sorting() {
    let mut differences = Vec::new();
    for sort in data()["sorts"].as_array().unwrap() {
        let pool = sort["pool"].as_str().unwrap();
        let elements = value(&sort["elements"]);
        let orderings = sort["orderings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| {
                (
                    o[0].as_str().unwrap().to_owned(),
                    direction(o[1].as_str().unwrap(), o[2].as_str().unwrap()),
                )
            })
            .collect();
        let actual = ArrayCollection::new(elements.as_array().unwrap().clone())
            .matching(&Criteria::new(None, Some(orderings), 0, None))
            .map(|c| Value::Array(c.get_keys()));
        let expected = outcome(&sort["result"]).unwrap();
        if !CONSISTENT.contains(&pool) {
            let sorted = |keys: &Value| {
                let mut keys: Vec<String> = keys
                    .as_array()
                    .unwrap()
                    .values()
                    .map(|k| k.as_string().unwrap().to_string_lossy().into_owned())
                    .collect();
                keys.sort();
                keys
            };
            let Outcome::Value(expected) = &expected else {
                panic!("{pool}: PHP threw {expected:?}");
            };
            assert_eq!(sorted(expected), sorted(actual.as_ref().unwrap()), "{pool}");
            continue;
        }
        if !same(&expected, &actual) {
            differences.push(format!("{pool}: PHP {expected:?}, port {actual:?}"));
        }
    }
    assert!(differences.is_empty(), "{differences:#?}");
}

#[test]
fn slicing() {
    for slice in data()["slices"].as_array().unwrap() {
        let elements = value(&slice[0]).as_array().unwrap().clone();
        let offset = slice[1].as_i64().unwrap();
        let length = slice[2].as_i64();
        let c = ArrayCollection::new(elements.clone());
        let context = format!("{elements:?} {offset} {length:?}");
        assert_eq!(
            value(&slice[3]),
            Value::Array(c.slice(offset, length)),
            "{context}: slice()"
        );
        let matched = c
            .matching(&Criteria::new(None, None, offset, length))
            .unwrap();
        assert_eq!(
            value(&slice[4]),
            Value::Array(matched.to_array()),
            "{context}: matching()"
        );
    }
}

#[test]
fn messages() {
    let messages = &data()["messages"];
    check(
        "Order::from",
        &messages["Order::from"],
        &Order::from_value(b"asc").map(|o| Value::from(o.value())),
    );
    let mut c = ArrayCollection::new(php_value::php_array![i64::MAX => 1]);
    check(
        "add after PHP_INT_MAX",
        &messages["add after PHP_INT_MAX"],
        &c.add(Value::Int(2)).map(|()| Value::Null),
    );
    let text = ArrayCollection::default().to_string();
    let (class, hash) = text.split_once('@').unwrap();
    assert_eq!(32, hash.len());
    assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
    check(
        "toString",
        &messages["toString"],
        &Ok(Value::from(format!("{class}@<hash>"))),
    );
}
