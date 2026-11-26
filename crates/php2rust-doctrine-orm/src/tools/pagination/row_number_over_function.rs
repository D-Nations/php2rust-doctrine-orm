//! `RowNumberOverFunction`: the `ROW_NUMBER() OVER(ORDER BY ...)` node the
//! limit subquery output walker builds; it cannot be used in DQL.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/RowNumberOverFunction.php src/Tools/Pagination/Exception/RowNumberOverFunctionNotEnabled.php

use php_value::Thrown;

use crate::query::ast::{FunctionNode, OrderByClause};
use crate::query::parser::Parser;
use crate::query::sql_walker::SqlWalker;

/// RowNumberOverFunction
///
/// Provides ROW_NUMBER() OVER(ORDER BY...) construct for use in LimitSubqueryOutputWalker
#[derive(Debug, Clone)]
pub struct RowNumberOverFunction {
    pub name: String,
    /// `public OrderByClause $orderByClause` (uninitialized until set).
    pub order_by_clause: Option<OrderByClause>,
}

impl RowNumberOverFunction {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\RowNumberOverFunction";

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            order_by_clause: None,
        }
    }
}

/// `RowNumberOverFunctionNotEnabled::create()`.
pub fn row_number_over_function_not_enabled() -> Thrown {
    Thrown::new(
        "Doctrine\\ORM\\Tools\\Pagination\\Exception\\RowNumberOverFunctionNotEnabled",
        "The RowNumberOverFunction is not intended for, nor is it enabled for use in DQL.",
        0,
    )
}

impl FunctionNode for RowNumberOverFunction {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get_sql(&self, sql_walker: &mut dyn SqlWalker<'_>) -> Result<String, Thrown> {
        let order_by_clause = self.order_by_clause.as_ref().ok_or_else(|| {
            Thrown::new(
                "Error",
                format!(
                    "Typed property {}::$orderByClause must not be accessed before initialization",
                    Self::CLASS
                ),
                0,
            )
        })?;

        Ok(format!(
            "ROW_NUMBER() OVER({})",
            sql_walker.walk_order_by_clause(order_by_clause)?.trim()
        ))
    }

    /// @throws RowNumberOverFunctionNotEnabled
    fn parse(&mut self, _parser: &mut Parser<'_>) -> Result<(), Thrown> {
        Err(row_number_over_function_not_enabled())
    }
}
