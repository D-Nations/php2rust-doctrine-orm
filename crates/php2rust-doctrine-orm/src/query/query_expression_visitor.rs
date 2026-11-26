//! `QueryExpressionVisitor`: turns a `Criteria`'s expression tree into DQL
//! expressions (`Expr` objects) and the parameters they bind.
//!
//! Upstream: doctrine/orm src/Query/QueryExpressionVisitor.php

use php_value::{PhpString, Thrown, Value};
use php2rust_doctrine_collections::expr::{
    Comparison, CompositeExpression, ExpressionVisitor, Value as CriteriaValue,
};

use crate::query::expr::{self, Andx, Orx};
use crate::query::{Expr, Parameter, query_exception};

/// Converts Collection expressions to Query expressions.
#[derive(Debug, Clone)]
pub struct QueryExpressionVisitor {
    query_aliases: Vec<String>,
    expr: Expr,
    parameters: Vec<Parameter>,
}

/// `OPERATOR_MAP`.
fn convert_comparison_operator(criteria_operator: &str) -> Option<&'static str> {
    match criteria_operator {
        Comparison::GT => Some(expr::Comparison::GT),
        Comparison::GTE => Some(expr::Comparison::GTE),
        Comparison::LT => Some(expr::Comparison::LT),
        Comparison::LTE => Some(expr::Comparison::LTE),
        _ => None,
    }
}

/// `'%' . $value . '%'` and the like: PHP's string conversion of the value.
fn concat(prefix: &str, value: &Value, suffix: &str) -> Result<Value, Thrown> {
    let mut out = prefix.as_bytes().to_vec();
    out.extend_from_slice(expr::to_string(value)?.as_bytes());
    out.extend_from_slice(suffix.as_bytes());
    Ok(Value::String(PhpString::from(out)))
}

impl QueryExpressionVisitor {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\QueryExpressionVisitor";

    /// `new QueryExpressionVisitor($queryAliases)`.
    pub fn new(query_aliases: Vec<String>) -> Self {
        Self {
            query_aliases,
            expr: Expr::new(),
            parameters: Vec::new(),
        }
    }

    /// Gets bound parameters.
    /// Filled after `dispatch()`.
    pub fn get_parameters(&self) -> Vec<Parameter> {
        self.parameters.clone()
    }

    /// Clears parameters.
    pub fn clear_parameters(&mut self) {
        self.parameters.clear();
    }
}

impl ExpressionVisitor for QueryExpressionVisitor {
    type Output = Result<Value, Thrown>;

    fn walk_composite_expression(&mut self, expr: &CompositeExpression) -> Self::Output {
        let mut expression_list = Vec::new();
        for child in expr.get_expression_list() {
            expression_list.push(self.dispatch(child)?);
        }
        match expr.get_type() {
            CompositeExpression::TYPE_AND => {
                Ok(Andx::new(Value::Array(php_value::PhpArray::list(expression_list)))?.into())
            }
            CompositeExpression::TYPE_OR => {
                Ok(Orx::new(Value::Array(php_value::PhpArray::list(expression_list)))?.into())
            }
            CompositeExpression::TYPE_NOT => {
                // `$expressionList[0]`: a NOT composite has one expression.
                let first = expression_list.into_iter().next().unwrap_or(Value::Null);
                Ok(self.expr.not(first).into())
            }
            other => Err(Thrown::new(
                "RuntimeException",
                format!("Unknown composite {other}"),
                0,
            )),
        }
    }

    fn walk_comparison(&mut self, comparison: &Comparison) -> Self::Output {
        let Some(first_alias) = self.query_aliases.first() else {
            return Err(query_exception::new(
                "No aliases are set before invoking walkComparison().",
            ));
        };

        let mut field = format!("{first_alias}.{}", comparison.get_field());

        for alias in &self.query_aliases {
            if format!("{}.", comparison.get_field()).starts_with(&format!("{alias}.")) {
                comparison.get_field().clone_into(&mut field);
                break;
            }
        }

        let mut parameter_name = comparison.get_field().replace('.', "_");

        if self
            .parameters
            .iter()
            .any(|parameter| parameter.get_name() == parameter_name)
        {
            parameter_name = format!("{parameter_name}_{}", self.parameters.len());
        }

        let value = self.walk_value(comparison.get_value())?;
        let mut parameter = Parameter::new(parameter_name.clone(), value.clone(), None);
        let placeholder = format!(":{parameter_name}");

        match comparison.get_operator() {
            Comparison::IN => {
                self.parameters.push(parameter);
                Ok(self.expr.r#in(&field, placeholder)?.into())
            }
            Comparison::NIN => {
                self.parameters.push(parameter);
                Ok(self.expr.not_in(&field, placeholder)?.into())
            }
            // `Comparison::EQ` and `Comparison::IS` are both `=`.
            Comparison::EQ => {
                if value.is_null() {
                    return Ok(self.expr.is_null(&field).into());
                }
                self.parameters.push(parameter);
                Ok(self.expr.eq(field, placeholder).into())
            }
            Comparison::NEQ => {
                if value.is_null() {
                    return Ok(self.expr.is_not_null(&field).into());
                }
                self.parameters.push(parameter);
                Ok(self.expr.neq(field, placeholder).into())
            }
            Comparison::CONTAINS => {
                let type_ = parameter.get_type().clone();
                parameter.set_value(concat("%", parameter.get_value(), "%")?, Some(type_));
                self.parameters.push(parameter);
                Ok(self.expr.like(&field, placeholder).into())
            }
            Comparison::MEMBER_OF => {
                // `isMemberOf(string $x, string $y)` under strict types.
                let member = comparison.get_value().get_value();
                let Some(member) = member.as_string() else {
                    return Err(Thrown::new(
                        "TypeError",
                        format!(
                            "Doctrine\\ORM\\Query\\Expr::isMemberOf(): Argument #2 ($y) must be of type string, {} given",
                            member.debug_type()
                        ),
                        0,
                    ));
                };
                let member = String::from_utf8_lossy(member.as_bytes()).into_owned();
                Ok(self
                    .expr
                    .is_member_of(comparison.get_field(), &member)
                    .into())
            }
            Comparison::STARTS_WITH => {
                let type_ = parameter.get_type().clone();
                parameter.set_value(concat("", parameter.get_value(), "%")?, Some(type_));
                self.parameters.push(parameter);
                Ok(self.expr.like(&field, placeholder).into())
            }
            Comparison::ENDS_WITH => {
                let type_ = parameter.get_type().clone();
                parameter.set_value(concat("%", parameter.get_value(), "")?, Some(type_));
                self.parameters.push(parameter);
                Ok(self.expr.like(&field, placeholder).into())
            }
            other => {
                if let Some(operator) = convert_comparison_operator(other) {
                    self.parameters.push(parameter);
                    return Ok(expr::Comparison::new(field, operator, placeholder).into());
                }
                Err(Thrown::new(
                    "RuntimeException",
                    format!("Unknown comparison operator: {other}"),
                    0,
                ))
            }
        }
    }

    fn walk_value(&mut self, value: &CriteriaValue) -> Self::Output {
        Ok(value.get_value().clone())
    }
}
