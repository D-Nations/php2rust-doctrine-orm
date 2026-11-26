//! `Doctrine\ORM\Tools\Pagination`: the paginator's walkers.
//!
//! Upstream: none

#![allow(clippy::doc_markdown, reason = "upstream's doc comments")]

pub mod count_output_walker;
pub mod count_walker;
pub mod limit_subquery_output_walker;
pub mod limit_subquery_walker;
pub mod offset_paginator;
pub mod paginator;
pub mod paginator_query;
pub mod root_type_walker;
pub mod row_number_over_function;
pub mod where_in_walker;
pub mod window;
pub mod window_page;

use php_value::Thrown;

use crate::query::ast::IdentificationVariableDeclaration;

pub use count_output_walker::CountOutputWalker;
pub use count_walker::CountWalker;
pub use limit_subquery_output_walker::LimitSubqueryOutputWalker;
pub use limit_subquery_walker::LimitSubqueryWalker;
pub use offset_paginator::{OffsetPaginator, PaginatorInterface};
pub use paginator::Paginator;
pub use paginator_query::{PaginatorQuery, QueryOrBuilder, ResolvedQuery};
pub use root_type_walker::RootTypeWalker;
pub use row_number_over_function::RowNumberOverFunction;
pub use where_in_walker::WhereInWalker;
pub use window::Window;
pub use window_page::{Page, WindowPage};

/// `PaginatorInterface::HINT_ENABLE_DISTINCT`.
pub const HINT_ENABLE_DISTINCT: &str = "paginator.distinct.enable";

fn runtime_exception(message: impl Into<String>) -> Thrown {
    Thrown::new("RuntimeException", message.into(), 0)
}

/// `reset($from)->rangeVariableDeclaration->aliasIdentificationVariable`:
/// PHP's `Error` for a query without a FROM range variable.
fn root_alias(from: &[IdentificationVariableDeclaration]) -> Result<String, Thrown> {
    from.first()
        .and_then(|declaration| declaration.range_variable_declaration.as_ref())
        .map(|declaration| declaration.alias_identification_variable.clone())
        .ok_or_else(|| {
            Thrown::new(
                "Error",
                "Attempt to read property \"aliasIdentificationVariable\" on null",
                0,
            )
        })
}
