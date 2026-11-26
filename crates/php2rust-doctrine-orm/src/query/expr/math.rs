//! `Expr\Math`.
//!
//! Upstream: doctrine/orm src/Query/Expr/Math.php

use php_value::{PhpString, Thrown, Value};

use super::php::{self, expression_object};

/// Expression class for DQL math statements.
#[derive(Debug, Clone)]
pub struct Math {
    left_expr: Value,
    operator: String,
    right_expr: Value,
}

impl Math {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\Expr\\Math";

    /// Creates a mathematical expression with the given arguments.
    pub fn new(
        left_expr: impl Into<Value>,
        operator: impl Into<String>,
        right_expr: impl Into<Value>,
    ) -> Self {
        Self {
            left_expr: left_expr.into(),
            operator: operator.into(),
            right_expr: right_expr.into(),
        }
    }

    pub fn get_left_expr(&self) -> &Value {
        &self.left_expr
    }

    pub fn get_operator(&self) -> &str {
        &self.operator
    }

    pub fn get_right_expr(&self) -> &Value {
        &self.right_expr
    }

    fn properties_in_order(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("leftExpr", self.left_expr.clone()),
            ("operator", Value::from(self.operator.as_str())),
            ("rightExpr", self.right_expr.clone()),
        ]
    }

    /// `__toString()`: a `Math` operand is put in parentheses.
    pub fn try_to_string(&self) -> Result<PhpString, Thrown> {
        let operand = |expr: &Value| -> Result<Vec<u8>, Thrown> {
            let text = php::to_string(expr)?;
            let is_math = expr
                .as_object()
                .is_some_and(|o| o.downcast_ref::<Math>().is_some());
            Ok(if is_math {
                [b"(", text.as_bytes(), b")"].concat()
            } else {
                text.as_bytes().to_vec()
            })
        };
        // Adjusting Left Expression
        let left_expr = operand(&self.left_expr)?;
        // Adjusting Right Expression
        let right_expr = operand(&self.right_expr)?;
        Ok(php::concat(&[
            &left_expr,
            b" ",
            self.operator.as_bytes(),
            b" ",
            &right_expr,
        ]))
    }
}

expression_object!(Math);
