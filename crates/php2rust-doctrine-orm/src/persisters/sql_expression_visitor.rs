//! `SqlExpressionVisitor`: visits a `Criteria`'s expressions and generates
//! SQL WHERE conditions from them.
//!
//! Upstream: doctrine/orm src/Persisters/SqlExpressionVisitor.php

use php_value::{Thrown, Value};
use php2rust_doctrine_collections::expr::{
    Comparison, CompositeExpression, ExpressionVisitor, Value as CriteriaValue,
};

use crate::exception;
use crate::mapping::ClassMetadata;

use super::EntityPersister;

/// Visit Expressions and generate SQL WHERE conditions from them.
#[derive(Debug)]
pub struct SqlExpressionVisitor<'a> {
    persister: &'a dyn EntityPersister,
    class_metadata: &'a ClassMetadata,
}

impl<'a> SqlExpressionVisitor<'a> {
    pub fn new(persister: &'a dyn EntityPersister, class_metadata: &'a ClassMetadata) -> Self {
        Self {
            persister,
            class_metadata,
        }
    }
}

impl ExpressionVisitor for SqlExpressionVisitor<'_> {
    type Output = Result<String, Thrown>;

    /// Converts a comparison expression into the target query language
    /// output.
    fn walk_comparison(&mut self, comparison: &Comparison) -> Self::Output {
        let field = comparison.get_field();
        let value = comparison.get_value().get_value(); // shortcut for walkValue()

        let (is_association, class_name) = {
            let info = self.class_metadata.read();
            (
                info.association_mappings.contains_key(field),
                info.name.clone(),
            )
        };
        if is_association
            && !value.is_null()
            && !matches!(value, Value::Object(_))
            && ![Comparison::IN, Comparison::NIN].contains(&comparison.get_operator())
        {
            return Err(exception::matching_association_field_requires_object(
                &class_name,
                field,
            ));
        }

        self.persister.get_select_condition_statement_sql(
            field,
            value,
            None,
            Some(comparison.get_operator()),
        )
    }

    /// Converts a composite expression into the target query language
    /// output.
    fn walk_composite_expression(&mut self, expr: &CompositeExpression) -> Self::Output {
        let mut expression_list = Vec::new();
        for child in expr.get_expression_list() {
            expression_list.push(self.dispatch(child)?);
        }
        match expr.get_type() {
            CompositeExpression::TYPE_AND => Ok(format!("({})", expression_list.join(" AND "))),
            CompositeExpression::TYPE_OR => Ok(format!("({})", expression_list.join(" OR "))),
            CompositeExpression::TYPE_NOT => Ok(format!(
                "NOT ({})",
                expression_list.first().map_or("", String::as_str)
            )),
            other => Err(Thrown::new(
                "RuntimeException",
                format!("Unknown composite {other}"),
                0,
            )),
        }
    }

    /// Converts a value expression into the target query language part.
    fn walk_value(&mut self, _value: &CriteriaValue) -> Self::Output {
        Ok("?".to_owned())
    }
}
