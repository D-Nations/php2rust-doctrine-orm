//! `SqlValueVisitor`: extracts the values and types of a `Criteria`'s
//! expression, in the order `SqlExpressionVisitor` writes the
//! placeholders.
//!
//! Upstream: doctrine/orm src/Persisters/SqlValueVisitor.php

use php_value::{PhpString, Thrown, Value};
use php2rust_doctrine_collections::expr::{
    Comparison, CompositeExpression, ExpressionVisitor, Value as CriteriaValue,
};

use crate::query::expr::to_string;

/// One comparison as `getParamsAndTypes()` gives it: the field, the value
/// and the operator.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueType {
    pub field: String,
    pub value: Value,
    pub operator: String,
}

/// Extract the values from a criteria/expression.
#[derive(Debug, Clone, Default)]
pub struct SqlValueVisitor {
    values: Vec<Value>,
    types: Vec<ValueType>,
}

impl SqlValueVisitor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the Parameters and Types necessary for matching the last
    /// visited expression.
    pub fn get_params_and_types(&self) -> (Vec<Value>, Vec<ValueType>) {
        (self.values.clone(), self.types.clone())
    }

    /// Returns the value from a Comparison. In case of a CONTAINS
    /// comparison, the value is wrapped in %-signs, because it will be used
    /// in a LIKE clause.
    pub fn get_value_from_comparison(comparison: &Comparison) -> Result<Value, Thrown> {
        let value = comparison.get_value().get_value();
        let wrap = |prefix: &str, suffix: &str| -> Result<Value, Thrown> {
            let mut out = prefix.as_bytes().to_vec();
            out.extend_from_slice(to_string(value)?.as_bytes());
            out.extend_from_slice(suffix.as_bytes());
            Ok(Value::String(PhpString::from(out)))
        };
        match comparison.get_operator() {
            Comparison::CONTAINS => wrap("%", "%"),
            Comparison::STARTS_WITH => wrap("", "%"),
            Comparison::ENDS_WITH => wrap("%", ""),
            _ => Ok(value.clone()),
        }
    }
}

impl ExpressionVisitor for SqlValueVisitor {
    type Output = Result<(), Thrown>;

    /// Converts a comparison expression into the target query language
    /// output.
    fn walk_comparison(&mut self, comparison: &Comparison) -> Self::Output {
        let value = Self::get_value_from_comparison(comparison)?;
        self.values.push(value.clone());
        self.types.push(ValueType {
            field: comparison.get_field().to_owned(),
            value,
            operator: comparison.get_operator().to_owned(),
        });
        Ok(())
    }

    /// Converts a value expression into the target query language part.
    fn walk_value(&mut self, _value: &CriteriaValue) -> Self::Output {
        Ok(())
    }

    /// Converts a composite expression into the target query language
    /// output.
    fn walk_composite_expression(&mut self, expr: &CompositeExpression) -> Self::Output {
        for child in expr.get_expression_list() {
            self.dispatch(child)?;
        }
        Ok(())
    }
}
