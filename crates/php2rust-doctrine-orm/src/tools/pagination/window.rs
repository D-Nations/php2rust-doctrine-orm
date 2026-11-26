//! `Window`: an offset-based pagination position.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/Window.php

use std::sync::Arc;

use php_value::{PhpArray, PhpObject, Thrown, Value};

fn invalid_argument(message: &str) -> Thrown {
    Thrown::new("InvalidArgumentException", message, 0)
}

/// Represents an offset-based pagination position: a window over the result set.
///
/// Counterpart to Cursor, but deliberately not its mirror image. A cursor
/// is a pure position, so the page size belongs to the paginator; a window is a
/// position *and* a size, because offset pagination has to know how far to skip,
/// which is a function of the page size. It carries no opaque state and needs no
/// encoding: navigation between pages is plain arithmetic.
///
/// The window is passed explicitly to OffsetPaginator::paginate(), instead
/// of being derived implicitly from \Doctrine\ORM\Query::setFirstResult()
/// and \Doctrine\ORM\Query::setMaxResults() on the paginated query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    first_result: i64,
    max_results: i64,
}

impl Window {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\Window";

    /// `new Window($firstResult, $maxResults)`.
    pub fn new(first_result: i64, max_results: i64) -> Result<Self, Thrown> {
        if first_result < 0 {
            return Err(invalid_argument(
                "firstResult must be greater than or equal to 0.",
            ));
        }

        if max_results < 1 {
            return Err(invalid_argument(
                "maxResults must be greater than or equal to 1.",
            ));
        }

        Ok(Self {
            first_result,
            max_results,
        })
    }

    /// Builds a window from a 1-based page number and a page size.
    pub fn from_page_number_and_size(page_number: i64, page_size: i64) -> Result<Self, Thrown> {
        if page_number < 1 {
            return Err(invalid_argument(
                "pageNumber must be greater than or equal to 1.",
            ));
        }

        Self::new((page_number - 1).saturating_mul(page_size), page_size)
    }

    pub fn get_first_result(&self) -> i64 {
        self.first_result
    }

    pub fn get_max_results(&self) -> i64 {
        self.max_results
    }

    /// Returns the 1-based page number this window points at.
    pub fn get_page_number(&self) -> i64 {
        self.first_result / self.max_results + 1
    }

    /// Returns the window for the next page, keeping the same page size.
    pub fn next(&self) -> Result<Self, Thrown> {
        Self::new(
            self.first_result.saturating_add(self.max_results),
            self.max_results,
        )
    }

    /// Returns the window for the previous page, keeping the same page size and
    /// clamping the first result at 0.
    pub fn previous(&self) -> Result<Self, Thrown> {
        Self::new(
            0.max(self.first_result - self.max_results),
            self.max_results,
        )
    }
}

impl PhpObject for Window {
    fn class(&self) -> &str {
        Self::CLASS
    }

    fn properties(&self) -> PhpArray {
        let mut properties = PhpArray::new();
        properties.insert("firstResult", Value::Int(self.first_result));
        properties.insert("maxResults", Value::Int(self.max_results));
        properties
    }
}

impl From<Window> for Value {
    fn from(window: Window) -> Self {
        Value::Object(Arc::new(window))
    }
}
