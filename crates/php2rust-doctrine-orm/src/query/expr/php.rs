//! The PHP conversions the expression classes rely on: `(string)` of a
//! `mixed` value (an expression object's `__toString()`, PHP's `Error`
//! for an object that has none, `"Array"` for an array), `implode()`,
//! `iterator_to_array()`, and the exceptions they throw.
//!
//! Upstream: none

use php_value::{Key, PhpArray, PhpObject, PhpString, Thrown, Value};

use super::{Andx, Comparison, Composite, From, Func, GroupBy, Join, Literal, Math, OrderBy, Orx, Select};

/// An expression class: its `__toString()`, which can throw when a part
/// cannot be converted to a string.
pub trait DqlExpression {
    /// `__toString()`.
    fn try_to_string(&self) -> Result<PhpString, Thrown>;

    /// `(array) $expression`: every property in declaration order, the
    /// protected ones under PHP's mangled names (`"\0*\0name"`).
    fn array_cast(&self) -> PhpArray;
}

/// A protected property's key in `(array) $object`.
pub(crate) fn protected(name: &str) -> Key {
    Key::from_bytes(format!("\0*\0{name}").as_bytes())
}

/// An optional string property's value.
pub(crate) fn nullable(value: Option<&str>) -> Value {
    value.map_or(Value::Null, Value::from)
}

/// `(array) $value`: [`DqlExpression::array_cast`] for an expression,
/// `php_value::object::to_array()` otherwise.
pub(crate) fn to_array(value: &Value) -> PhpArray {
    match value {
        Value::Object(o) => match as_expression(o.as_ref()) {
            Some(expression) => expression.array_cast(),
            None => o.properties(),
        },
        other => php_value::object::to_array(other),
    }
}

/// The expression object behind `object`, if it is one of these classes.
pub(crate) fn as_expression(object: &dyn PhpObject) -> Option<&dyn DqlExpression> {
    macro_rules! downcast {
        ($($class:ty),*) => {
            $(if let Some(e) = object.downcast_ref::<$class>() {
                return Some(e);
            })*
        };
    }
    downcast!(Andx, Orx, Composite, Select, GroupBy, Literal, Comparison, Func, Math, From, Join, OrderBy);
    None
}

/// `(string) $value`: an expression's `__toString()`, PHP's `Error` for
/// an object without one, `"Array"` for an array (PHP also warns).
pub fn to_string(value: &Value) -> Result<PhpString, Thrown> {
    match value {
        Value::Object(object) => {
            if let Some(expression) = as_expression(object.as_ref()) {
                return expression.try_to_string();
            }
            object.to_php_string().ok_or_else(|| {
                Thrown::new(
                    "Error",
                    format!(
                        "Object of class {} could not be converted to string",
                        object.class()
                    ),
                    0,
                )
            })
        }
        // PHP warns "Array to string conversion".
        Value::Array(_) => Ok(PhpString::from("Array")),
        scalar => Ok(scalar.to_php_string().unwrap_or_default()),
    }
}

/// `implode($separator, $values)`.
pub(crate) fn implode<'a>(
    separator: &str,
    values: impl IntoIterator<Item = &'a Value>,
) -> Result<PhpString, Thrown> {
    let mut out = Vec::new();
    for (i, value) in values.into_iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(separator.as_bytes());
        }
        out.extend_from_slice(to_string(value)?.as_bytes());
    }
    Ok(PhpString::from(out))
}

/// Concatenates byte strings into a PHP string.
pub(crate) fn concat(parts: &[&[u8]]) -> PhpString {
    PhpString::from(parts.concat())
}

/// `iterator_to_array($traversable)` (keys preserved): `None` when the
/// object is not `\Traversable`.
pub(crate) fn iterator_to_array(object: &dyn PhpObject) -> Option<Result<PhpArray, Thrown>> {
    let pairs = match object.traverse()? {
        Ok(pairs) => pairs,
        Err(thrown) => return Some(Err(thrown)),
    };
    let mut array = PhpArray::new();
    for (key, value) in pairs {
        let key = match key {
            Value::Int(i) => Key::from(i),
            Value::String(s) => Key::from(s),
            Value::Null => Key::from(""),
            Value::Bool(b) => Key::from(b),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "PHP truncates a float key to an int"
            )]
            Value::Float(f) => Key::from(f as i64),
            other => {
                return Some(Err(Thrown::new(
                    "TypeError",
                    format!("Cannot access offset of type {} on array", other.debug_type()),
                    0,
                )));
            }
        };
        array.insert(key, value);
    }
    Some(Ok(array))
}

/// PHP's truthiness of a string (`""` and `"0"` are false).
pub(crate) fn string_is_truthy(s: &str) -> bool {
    !s.is_empty() && s != "0"
}

/// The `InvalidArgumentException` PHP throws.
pub(crate) fn invalid_argument(message: String) -> Thrown {
    Thrown::new("InvalidArgumentException", message, 0)
}

/// Declares an expression class (not a subclass of `Base`) as a
/// `php_value` object: `\Stringable` through its `try_to_string()`.
macro_rules! expression_object {
    ($name:ident) => {
        impl $crate::query::expr::DqlExpression for $name {
            fn try_to_string(&self) -> Result<php_value::PhpString, php_value::Thrown> {
                $name::try_to_string(self)
            }

            fn array_cast(&self) -> php_value::PhpArray {
                self.properties_in_order()
                    .into_iter()
                    .map(|(name, value)| ($crate::query::expr::php::protected(name), value))
                    .collect()
            }
        }

        impl php_value::PhpObject for $name {
            fn class(&self) -> &str {
                Self::CLASS
            }

            fn properties(&self) -> php_value::PhpArray {
                php_value::PhpArray::new()
            }

            fn to_php_string(&self) -> Option<php_value::PhpString> {
                self.try_to_string().ok()
            }

            fn is_stringable(&self) -> bool {
                true
            }

            fn instance_of(&self, class: &str) -> bool {
                class.eq_ignore_ascii_case("Stringable")
            }
        }

        impl ::core::convert::From<$name> for php_value::Value {
            fn from(expression: $name) -> Self {
                php_value::Value::Object(std::sync::Arc::new(expression))
            }
        }
    };
}

pub(crate) use expression_object;
