//! Upstream: doctrine/collections src/Expr/ClosureExpressionVisitor.php

use std::fmt;
use std::sync::Arc;

use php_value::ops::{compare, to_string};
use php_value::{Key, Thrown, Value as PhpValue};

use super::{Comparison, CompositeExpression, ExpressionVisitor, Value};
use crate::php::{array_filter_callback, in_array, iterator_to_array};
use crate::{deprecations, reflection};

/// A closure `walkComparison()` or `walkCompositeExpression()` builds:
/// whether an element (an object or an array) matches.
pub type Filter = Arc<dyn Fn(&PhpValue) -> Result<bool, Thrown> + Send + Sync>;

/// A closure `sortByField()` builds: PHP's comparison result (`-1`, `0`,
/// `1`, times the orientation) of two elements.
pub type Comparator = Arc<dyn Fn(&PhpValue, &PhpValue) -> Result<i64, Thrown> + Send + Sync>;

/// What `ClosureExpressionVisitor` returns for a node: a closure for a
/// comparison or a composite expression, the value itself for a `Value`.
#[derive(Clone)]
pub enum Walked {
    Closure(Filter),
    Value(PhpValue),
}

impl Walked {
    /// Whether `array_filter($elements, $walked)` keeps `element`: the
    /// closure's answer, or for a value its call as `array_filter()`'s
    /// callback (`null` keeps the truthy elements).
    pub fn accepts(&self, element: &PhpValue) -> Result<bool, Thrown> {
        match self {
            Self::Closure(filter) => filter(element),
            Self::Value(callback) => array_filter_callback(callback, element),
        }
    }
}

impl fmt::Debug for Walked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closure(_) => f.write_str("Closure"),
            Self::Value(value) => f.debug_tuple("Value").field(value).finish(),
        }
    }
}

const CLASS: &str = "Doctrine\\Common\\Collections\\Expr\\ClosureExpressionVisitor";

/// `ClosureExpressionVisitor`: turns an expression into a PHP-style
/// filter closure over objects and arrays.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClosureExpressionVisitor {
    /// Deprecated and a no-op, kept for BC.
    #[allow(dead_code)]
    access_raw_field_values: bool,
}

/// A `TypeError` for the argument `#1 ($object)` of a user function or
/// closure named `function`. PHP's message goes on with `, called in
/// <file> on line <line>`, which is left out.
fn object_type_error(function: &str, wanted: &str, given: &PhpValue) -> Thrown {
    Thrown::new(
        "TypeError",
        format!(
            "{function}(): Argument #1 ($object) must be of type {wanted}, {} given",
            given.value_name()
        ),
        0,
    )
}

fn runtime_exception(message: String) -> Thrown {
    Thrown::new("RuntimeException", message, 0)
}

/// `(string) $value` as `str_contains()` and friends receive it.
fn string_of(value: &PhpValue) -> Result<Vec<u8>, Thrown> {
    Ok(to_string(value)?.as_bytes().to_vec())
}

impl ClosureExpressionVisitor {
    /// `new ClosureExpressionVisitor($accessRawFieldValues)`; the argument
    /// is deprecated and a no-op.
    pub fn new(access_raw_field_values: bool) -> Self {
        Self {
            access_raw_field_values,
        }
    }

    /// `getObjectFieldValue($object, $field)`: `a.b` reads `b` of the value
    /// of `a`; an array's key (`null` for a missing one, where PHP warns),
    /// or an object's property of any visibility, found up its parent
    /// classes and read raw (`ReflectionProperty::getRawValue()`, see
    /// [`crate::reflection`]); a `RuntimeException` for a property no class
    /// has, a `TypeError` for a value that is neither an object nor an
    /// array.
    pub fn get_object_field_value(object: &PhpValue, field: &str) -> Result<PhpValue, Thrown> {
        if let Some((field, sub_field)) = field.split_once('.') {
            let object = Self::get_object_field_value(object, field)?;
            return Self::get_object_field_value(&object, sub_field);
        }
        match object {
            PhpValue::Array(array) => Ok(array.get(&Key::from(field)).cloned().unwrap_or_default()),
            PhpValue::Object(o) => match reflection::raw_property(o.as_ref(), field) {
                Some(value) => value,
                None => Err(runtime_exception(format!(
                    "Field \"{field}\" does not exist in class \"{}\"",
                    o.class()
                ))),
            },
            other => Err(object_type_error(
                &format!("{CLASS}::getObjectFieldValue"),
                "object|array",
                other,
            )),
        }
    }

    /// `getObjectFieldValue($object, $field, $accessRawFieldValues)`: the
    /// third argument is deprecated (a deprecation is triggered) and a
    /// no-op.
    pub fn get_object_field_value_legacy(
        object: &PhpValue,
        field: &str,
        access_raw_field_values: bool,
    ) -> Result<PhpValue, Thrown> {
        let _ = access_raw_field_values;
        deprecations::trigger_access_raw_field_values(&format!("{CLASS}::getObjectFieldValue"));
        Self::get_object_field_value(object, field)
    }

    /// `sortByField($name, $orientation, $next)`: a comparator by the
    /// field `name`; elements whose values are identical (`===`) go to
    /// `next` (or compare equal), others compare by `>` times
    /// `orientation`.
    pub fn sort_by_field(name: &str, orientation: i64, next: Option<Comparator>) -> Comparator {
        let next = next.unwrap_or_else(|| Arc::new(|_: &PhpValue, _: &PhpValue| Ok(0)));
        let name = name.to_owned();
        Arc::new(move |a: &PhpValue, b: &PhpValue| {
            let a_value = Self::get_object_field_value(a, &name)?;
            let b_value = Self::get_object_field_value(b, &name)?;
            if a_value.identical(&b_value) {
                return next(a, b);
            }
            // `$aValue > $bValue` is `$bValue < $aValue`.
            let greater = compare(&b_value, &a_value).is_lt();
            Ok((if greater { 1i64 } else { -1 }).saturating_mul(orientation))
        })
    }

    /// `sortByField($name, $orientation, $next, $accessRawFieldValues)`:
    /// the fourth argument is deprecated (a deprecation is triggered) and
    /// a no-op.
    pub fn sort_by_field_legacy(
        name: &str,
        orientation: i64,
        next: Option<Comparator>,
        access_raw_field_values: bool,
    ) -> Comparator {
        let _ = access_raw_field_values;
        deprecations::trigger_access_raw_field_values(&format!("{CLASS}::sortByField"));
        Self::sort_by_field(name, orientation, next)
    }

    /// `walkComparison($comparison)`: the closure for the comparison's
    /// operator; a `RuntimeException` for an unknown operator.
    pub fn walk_comparison(&self, comparison: &Comparison) -> Result<Filter, Thrown> {
        let field = comparison.get_field().to_owned();
        let value = comparison.get_value().get_value().clone();
        let get = move |object: &PhpValue| Self::get_object_field_value(object, &field);
        let filter: Filter = match comparison.get_operator() {
            Comparison::EQ => Arc::new(move |o| Ok(get(o)?.identical(&value))),
            Comparison::NEQ => Arc::new(move |o| Ok(!get(o)?.identical(&value))),
            Comparison::LT => Arc::new(move |o| Ok(compare(&get(o)?, &value).is_lt())),
            Comparison::LTE => Arc::new(move |o| Ok(compare(&get(o)?, &value).is_le())),
            // `$a > $b` and `$a >= $b` are `$b < $a` and `$b <= $a`.
            Comparison::GT => Arc::new(move |o| Ok(compare(&value, &get(o)?).is_lt())),
            Comparison::GTE => Arc::new(move |o| Ok(compare(&value, &get(o)?).is_le())),
            Comparison::IN => Arc::new(move |o| {
                let field_value = get(o)?;
                Ok(in_array(
                    &field_value,
                    haystack(&value)?,
                    is_scalar(&field_value),
                ))
            }),
            Comparison::NIN => Arc::new(move |o| {
                let field_value = get(o)?;
                Ok(!in_array(
                    &field_value,
                    haystack(&value)?,
                    is_scalar(&field_value),
                ))
            }),
            Comparison::CONTAINS => Arc::new(move |o| {
                let haystack = string_of(&get(o)?)?;
                let needle = string_of(&value)?;
                Ok(needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle))
            }),
            Comparison::MEMBER_OF => Arc::new(move |o| {
                let field_values = iterator_to_array(&get(o)?)?;
                Ok(in_array(&value, &field_values, true))
            }),
            Comparison::STARTS_WITH => Arc::new(move |o| {
                let haystack = string_of(&get(o)?)?;
                Ok(haystack.starts_with(&string_of(&value)?))
            }),
            Comparison::ENDS_WITH => Arc::new(move |o| {
                let haystack = string_of(&get(o)?)?;
                Ok(haystack.ends_with(&string_of(&value)?))
            }),
            op => {
                return Err(runtime_exception(format!(
                    "Unknown comparison operator: {op}"
                )));
            }
        };
        Ok(filter)
    }

    /// `walkValue($value)`: the value itself.
    pub fn walk_value(&self, value: &Value) -> PhpValue {
        value.get_value().clone()
    }

    /// `walkCompositeExpression($expr)`: the children's closures joined by
    /// `AND`, `OR` or `NOT`; a `RuntimeException` for another type. The
    /// joined closures take objects only (`object $object`): an array is
    /// a `TypeError`.
    pub fn walk_composite_expression(&self, expr: &CompositeExpression) -> Result<Filter, Thrown> {
        let mut expression_list = Vec::new();
        for child in expr.get_expression_list() {
            let mut visitor = *self;
            expression_list.push(visitor.dispatch(child)?);
        }
        match expr.get_type() {
            CompositeExpression::TYPE_AND => Ok(Self::and_expressions(expression_list)),
            CompositeExpression::TYPE_OR => Ok(Self::or_expressions(expression_list)),
            CompositeExpression::TYPE_NOT => Ok(Self::not_expression(expression_list)),
            type_ => Err(runtime_exception(format!("Unknown composite {type_}"))),
        }
    }

    fn and_expressions(expressions: Vec<Walked>) -> Filter {
        Arc::new(move |object| {
            require_object(object, "andExpressions", 183)?;
            for expression in &expressions {
                if !expression.accepts(object)? {
                    return Ok(false);
                }
            }
            Ok(true)
        })
    }

    fn or_expressions(expressions: Vec<Walked>) -> Filter {
        Arc::new(move |object| {
            require_object(object, "orExpressions", 196)?;
            for expression in &expressions {
                if expression.accepts(object)? {
                    return Ok(true);
                }
            }
            Ok(false)
        })
    }

    fn not_expression(expressions: Vec<Walked>) -> Filter {
        Arc::new(move |object| {
            require_object(object, "notExpression", 209)?;
            match expressions.first() {
                Some(expression) => Ok(!expression.accepts(object)?),
                // CompositeExpression gives NOT exactly one child.
                None => Ok(true),
            }
        })
    }
}

/// The `object $object` parameter of a composite closure, declared at
/// `line` of upstream's file in `method`.
fn require_object(object: &PhpValue, method: &str, line: u32) -> Result<(), Thrown> {
    if matches!(object, PhpValue::Object(_)) {
        return Ok(());
    }
    Err(object_type_error(
        &format!("{CLASS}::{{closure:{CLASS}::{method}():{line}}}"),
        "object",
        object,
    ))
}

/// `is_scalar($value)`.
fn is_scalar(value: &PhpValue) -> bool {
    matches!(
        value,
        PhpValue::Bool(_) | PhpValue::Int(_) | PhpValue::Float(_) | PhpValue::String(_)
    )
}

/// The `$haystack` of `in_array()`: a `TypeError` for a value that is not
/// an array.
fn haystack(value: &PhpValue) -> Result<&php_value::PhpArray, Thrown> {
    value.as_array().ok_or_else(|| {
        Thrown::new(
            "TypeError",
            format!(
                "in_array(): Argument #2 ($haystack) must be of type array, {} given",
                value.value_name()
            ),
            0,
        )
    })
}

impl ExpressionVisitor for ClosureExpressionVisitor {
    type Output = Result<Walked, Thrown>;

    fn walk_comparison(&mut self, comparison: &Comparison) -> Self::Output {
        ClosureExpressionVisitor::walk_comparison(self, comparison).map(Walked::Closure)
    }

    fn walk_value(&mut self, value: &Value) -> Self::Output {
        Ok(Walked::Value(ClosureExpressionVisitor::walk_value(
            self, value,
        )))
    }

    fn walk_composite_expression(&mut self, expr: &CompositeExpression) -> Self::Output {
        ClosureExpressionVisitor::walk_composite_expression(self, expr).map(Walked::Closure)
    }
}
