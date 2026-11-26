//! `Expr`, the DQL expression builder, and its expression classes
//! (`Doctrine\ORM\Query\Expr\*`).
//!
//! PHP's `mixed` arguments are `php_value::Value`s; an expression is
//! passed as a value with `Value::from(expression)` (a `php_value`
//! object, so a `Composite` part keeps its own handle, as in PHP). A
//! `__toString()` that converts a part which has no string form returns
//! PHP's `Error`, so the expressions convert with `try_to_string()`.
//!
//! Upstream: doctrine/orm src/Query/Expr.php

#![allow(clippy::doc_markdown, reason = "upstream's doc comments")]

mod andx;
mod base;
mod comparison;
mod composite;
mod from;
mod func;
mod group_by;
mod join;
mod literal;
mod math;
mod order_by;
mod orx;
mod php;
mod select;

use php_value::{PhpArray, PhpString, Thrown, Value};

pub use andx::Andx;
pub use comparison::Comparison;
pub use composite::Composite;
pub use from::From;
pub use func::Func;
pub use group_by::GroupBy;
pub use join::Join;
pub use literal::Literal;
pub use math::Math;
pub use order_by::{ORDER_DEPRECATION, Order, OrderBy};
pub use orx::Orx;
pub use php::{DqlExpression, to_string};
pub use select::Select;

use crate::query::SortDirection;

/// This class is used to generate DQL expressions via a set of PHP static functions.
///
/// The variadic methods take a `Vec`; PHP's check for unknown named
/// arguments (`NoUnknownNamedArguments`) has no counterpart, as Rust has
/// no named arguments.
#[derive(Debug, Clone, Copy, Default)]
pub struct Expr;

fn list(values: Vec<Value>) -> Value {
    Value::Array(PhpArray::list(values))
}

impl Expr {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\Expr";

    pub fn new() -> Self {
        Self
    }

    /// `andX()`'s and `orX()`'s parameter type
    /// (`Comparison|Func|Andx|Orx|string`), as PHP checks it for a caller
    /// without `declare(strict_types=1)`: a scalar or another
    /// `\Stringable` object becomes its string, and anything else is a
    /// `TypeError`.
    fn boolean_expressions(method: &str, x: Vec<Value>) -> Result<Value, Thrown> {
        let mut checked = Vec::with_capacity(x.len());
        for (i, arg) in x.into_iter().enumerate() {
            let arg = match arg {
                Value::String(_) => arg,
                Value::Object(ref o)
                    if o.downcast_ref::<Comparison>().is_some()
                        || o.downcast_ref::<Func>().is_some()
                        || o.downcast_ref::<Andx>().is_some()
                        || o.downcast_ref::<Orx>().is_some() =>
                {
                    arg
                }
                Value::Int(_) | Value::Float(_) | Value::Bool(_) => {
                    Value::String(arg.to_php_string().unwrap_or_default())
                }
                Value::Object(ref o)
                    if o.is_stringable() || php::as_expression(o.as_ref()).is_some() =>
                {
                    Value::String(php::to_string(&arg)?)
                }
                other => {
                    return Err(Thrown::new(
                        "TypeError",
                        format!(
                            "{}::{method}(): Argument #{} must be of type {}|{}|{}|{}|string, {} given",
                            Self::CLASS,
                            i + 1,
                            Comparison::CLASS,
                            Func::CLASS,
                            Andx::CLASS,
                            Orx::CLASS,
                            other.value_name()
                        ),
                        0,
                    ));
                }
            };
            checked.push(arg);
        }
        Ok(list(checked))
    }

    /// Creates a conjunction of the given boolean expressions
    /// (`Comparison`, `Func`, `Andx`, `Orx` or strings; see
    /// [`Expr::boolean_expressions`] for other values).
    pub fn and_x(&self, x: Vec<Value>) -> Result<Andx, Thrown> {
        Andx::new(Self::boolean_expressions("andX", x)?)
    }

    /// Creates a disjunction of the given boolean expressions
    /// (`Comparison`, `Func`, `Andx`, `Orx` or strings; see
    /// [`Expr::boolean_expressions`] for other values).
    pub fn or_x(&self, x: Vec<Value>) -> Result<Orx, Thrown> {
        Orx::new(Self::boolean_expressions("orX", x)?)
    }

    /// Creates an ASCending order expression.
    pub fn asc(&self, expr: &str) -> OrderBy {
        OrderBy::new(Some(expr), SortDirection::Ascending)
    }

    /// Creates a DESCending order expression.
    pub fn desc(&self, expr: &str) -> OrderBy {
        OrderBy::new(Some(expr), SortDirection::Descending)
    }

    /// Creates an equality comparison expression with the given arguments.
    pub fn eq(&self, x: impl Into<Value>, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, Comparison::EQ, y)
    }

    /// Creates an instance of Expr\Comparison, with the given arguments.
    pub fn neq(&self, x: impl Into<Value>, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, Comparison::NEQ, y)
    }

    /// Creates an instance of Expr\Comparison, with the given arguments.
    pub fn lt(&self, x: impl Into<Value>, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, Comparison::LT, y)
    }

    /// Creates an instance of Expr\Comparison, with the given arguments.
    pub fn lte(&self, x: impl Into<Value>, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, Comparison::LTE, y)
    }

    /// Creates an instance of Expr\Comparison, with the given arguments.
    pub fn gt(&self, x: impl Into<Value>, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, Comparison::GT, y)
    }

    /// Creates an instance of Expr\Comparison, with the given arguments.
    pub fn gte(&self, x: impl Into<Value>, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, Comparison::GTE, y)
    }

    /// Creates an instance of AVG() function, with the given argument.
    pub fn avg(&self, x: impl Into<Value>) -> Func {
        Func::new("AVG", list(vec![x.into()]))
    }

    /// Creates an instance of MAX() function, with the given argument.
    pub fn max(&self, x: impl Into<Value>) -> Func {
        Func::new("MAX", list(vec![x.into()]))
    }

    /// Creates an instance of MIN() function, with the given argument.
    pub fn min(&self, x: impl Into<Value>) -> Func {
        Func::new("MIN", list(vec![x.into()]))
    }

    /// Creates an instance of COUNT() function, with the given argument.
    pub fn count(&self, x: impl Into<Value>) -> Func {
        Func::new("COUNT", list(vec![x.into()]))
    }

    /// Creates an instance of COUNT(DISTINCT) function, with the given argument.
    pub fn count_distinct(&self, x: &[Value]) -> Result<PhpString, Thrown> {
        Ok(php::concat(&[
            b"COUNT(DISTINCT ",
            php::implode(", ", x)?.as_bytes(),
            b")",
        ]))
    }

    /// Creates an instance of EXISTS() function, with the given DQL Subquery.
    pub fn exists(&self, subquery: impl Into<Value>) -> Func {
        Func::new("EXISTS", list(vec![subquery.into()]))
    }

    /// Creates an instance of ALL() function, with the given DQL Subquery.
    pub fn all(&self, subquery: impl Into<Value>) -> Func {
        Func::new("ALL", list(vec![subquery.into()]))
    }

    /// Creates a SOME() function expression with the given DQL subquery.
    pub fn some(&self, subquery: impl Into<Value>) -> Func {
        Func::new("SOME", list(vec![subquery.into()]))
    }

    /// Creates an ANY() function expression with the given DQL subquery.
    pub fn any(&self, subquery: impl Into<Value>) -> Func {
        Func::new("ANY", list(vec![subquery.into()]))
    }

    /// Creates a negation expression of the given restriction.
    pub fn not(&self, restriction: impl Into<Value>) -> Func {
        Func::new("NOT", list(vec![restriction.into()]))
    }

    /// Creates an ABS() function expression with the given argument.
    pub fn abs(&self, x: impl Into<Value>) -> Func {
        Func::new("ABS", list(vec![x.into()]))
    }

    /// Creates a MOD($x, $y) function expression to return the remainder of $x divided by $y.
    pub fn r#mod(&self, x: impl Into<Value>, y: impl Into<Value>) -> Func {
        Func::new("MOD", list(vec![x.into(), y.into()]))
    }

    /// Creates a product mathematical expression with the given arguments.
    pub fn prod(&self, x: impl Into<Value>, y: impl Into<Value>) -> Math {
        Math::new(x, "*", y)
    }

    /// Creates a difference mathematical expression with the given arguments.
    pub fn diff(&self, x: impl Into<Value>, y: impl Into<Value>) -> Math {
        Math::new(x, "-", y)
    }

    /// Creates a sum mathematical expression with the given arguments.
    pub fn sum(&self, x: impl Into<Value>, y: impl Into<Value>) -> Math {
        Math::new(x, "+", y)
    }

    /// Creates a quotient mathematical expression with the given arguments.
    pub fn quot(&self, x: impl Into<Value>, y: impl Into<Value>) -> Math {
        Math::new(x, "/", y)
    }

    /// Creates a SQRT() function expression with the given argument.
    pub fn sqrt(&self, x: impl Into<Value>) -> Func {
        Func::new("SQRT", list(vec![x.into()]))
    }

    /// The elements of an iterable `$y`, each quoted unless it is a
    /// `Literal`; any other value as it is.
    fn quote_iterable(y: Value) -> Result<Value, Thrown> {
        let array = match &y {
            Value::Array(array) => array.clone(),
            Value::Object(object) => match php::iterator_to_array(object.as_ref()) {
                Some(array) => array?,
                None => return Ok(y),
            },
            _ => return Ok(y),
        };
        let mut quoted = PhpArray::new();
        for (key, literal) in array.iter() {
            let is_literal = literal
                .as_object()
                .is_some_and(|o| o.downcast_ref::<Literal>().is_some());
            let literal = if is_literal {
                literal.clone()
            } else {
                Value::String(Self::quote_literal(literal)?)
            };
            quoted.insert(key.clone(), literal);
        }
        Ok(Value::Array(quoted))
    }

    /// Creates an IN() expression with the given arguments.
    pub fn r#in(&self, x: &str, y: impl Into<Value>) -> Result<Func, Thrown> {
        Ok(Func::new(format!("{x} IN"), Self::quote_iterable(y.into())?))
    }

    /// Creates a NOT IN() expression with the given arguments.
    pub fn not_in(&self, x: &str, y: impl Into<Value>) -> Result<Func, Thrown> {
        Ok(Func::new(format!("{x} NOT IN"), Self::quote_iterable(y.into())?))
    }

    /// Creates an IS NULL expression with the given arguments.
    pub fn is_null(&self, x: &str) -> String {
        format!("{x} IS NULL")
    }

    /// Creates an IS NOT NULL expression with the given arguments.
    pub fn is_not_null(&self, x: &str) -> String {
        format!("{x} IS NOT NULL")
    }

    /// Creates a LIKE() comparison expression with the given arguments.
    pub fn like(&self, x: &str, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, "LIKE", y)
    }

    /// Creates a NOT LIKE() comparison expression with the given arguments.
    pub fn not_like(&self, x: &str, y: impl Into<Value>) -> Comparison {
        Comparison::new(x, "NOT LIKE", y)
    }

    /// Creates a CONCAT() function expression with the given arguments.
    pub fn concat(&self, x: Vec<Value>) -> Func {
        Func::new("CONCAT", list(x))
    }

    /// Creates a SUBSTRING() function expression with the given arguments.
    pub fn substring(&self, x: impl Into<Value>, from: i64, len: Option<i64>) -> Func {
        let mut args = vec![x.into(), Value::Int(from)];
        if let Some(len) = len {
            args.push(Value::Int(len));
        }
        Func::new("SUBSTRING", list(args))
    }

    /// Creates a LOWER() function expression with the given argument.
    pub fn lower(&self, x: impl Into<Value>) -> Func {
        Func::new("LOWER", list(vec![x.into()]))
    }

    /// Creates an UPPER() function expression with the given argument.
    pub fn upper(&self, x: impl Into<Value>) -> Func {
        Func::new("UPPER", list(vec![x.into()]))
    }

    /// Creates a LENGTH() function expression with the given argument.
    pub fn length(&self, x: impl Into<Value>) -> Func {
        Func::new("LENGTH", list(vec![x.into()]))
    }

    /// Creates a literal expression of the given argument (a bool, string,
    /// int or float; a `\Stringable` object becomes its string, as for a
    /// caller without `declare(strict_types=1)`; PHP's `TypeError` for
    /// anything else).
    pub fn literal(&self, literal: impl Into<Value>) -> Result<Literal, Thrown> {
        let literal = match literal.into() {
            Value::Object(o) if o.is_stringable() || php::as_expression(o.as_ref()).is_some() => {
                Value::String(php::to_string(&Value::Object(o))?)
            }
            other => other,
        };
        Literal::new(Value::String(Self::quote_literal(&literal)?))
    }

    /// Quotes a literal value, if necessary, according to the DQL syntax.
    fn quote_literal(literal: &Value) -> Result<PhpString, Thrown> {
        match literal {
            Value::Int(_) | Value::Float(_) => Ok(literal.to_php_string().unwrap_or_default()),
            Value::Bool(b) => Ok(PhpString::from(if *b { "true" } else { "false" })),
            Value::String(s) => {
                let mut out = vec![b'\''];
                for &b in s.as_bytes() {
                    if b == b'\'' {
                        out.extend_from_slice(b"''");
                    } else {
                        out.push(b);
                    }
                }
                out.push(b'\'');
                Ok(PhpString::from(out))
            }
            other => Err(Thrown::new(
                "TypeError",
                format!(
                    "{}::quoteLiteral(): Argument #1 ($literal) must be of type string|int|float|bool, {} given",
                    Self::CLASS,
                    other.value_name()
                ),
                0,
            )),
        }
    }

    /// Creates an instance of BETWEEN() function, with the given argument
    /// (`$x` and `$y` are ints or strings).
    pub fn between(
        &self,
        val: impl Into<Value>,
        x: impl Into<Value>,
        y: impl Into<Value>,
    ) -> Result<PhpString, Thrown> {
        Ok(php::concat(&[
            php::to_string(&val.into())?.as_bytes(),
            b" BETWEEN ",
            php::to_string(&x.into())?.as_bytes(),
            b" AND ",
            php::to_string(&y.into())?.as_bytes(),
        ]))
    }

    /// Creates an instance of TRIM() function, with the given argument.
    pub fn trim(&self, x: impl Into<Value>) -> Func {
        Func::new("TRIM", x)
    }

    /// Creates an instance of MEMBER OF function, with the given arguments.
    pub fn is_member_of(&self, x: &str, y: &str) -> Comparison {
        Comparison::new(x, "MEMBER OF", y)
    }

    /// Creates an instance of INSTANCE OF function, with the given arguments.
    pub fn is_instance_of(&self, x: &str, y: &str) -> Comparison {
        Comparison::new(x, "INSTANCE OF", y)
    }
}
