//! Upstream: doctrine/collections src/Expr/Value.php

use php_value::{PhpArray, PhpString, Value as PhpValue};

use super::ExpressionVisitor;

/// `Value`: a PHP value as an expression (the right side of a
/// `Comparison`, or a callable criteria).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Value {
    value: PhpValue,
}

impl Value {
    pub fn new(value: impl Into<PhpValue>) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// `getValue()`.
    pub fn get_value(&self) -> &PhpValue {
        &self.value
    }

    /// `visit($visitor)`: `$visitor->walkValue($this)`.
    pub fn visit<V: ExpressionVisitor + ?Sized>(&self, visitor: &mut V) -> V::Output {
        visitor.walk_value(self)
    }
}

impl From<PhpValue> for Value {
    fn from(value: PhpValue) -> Self {
        Self { value }
    }
}

macro_rules! from_php {
    ($($t:ty),*) => {$(
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                Self { value: PhpValue::from(value) }
            }
        }
    )*};
}

from_php!(bool, i64, f64, &str, String, PhpString, PhpArray);
