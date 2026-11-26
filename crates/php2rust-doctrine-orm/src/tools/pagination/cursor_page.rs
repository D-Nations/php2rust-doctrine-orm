//! `CursorPage`: the immutable result of a cursor pagination.
//!
//! The COUNT query and the cursor of an item need the database (the DBAL
//! is async), so `getTotalCount()`, the cursor getters and
//! `getItemsWithCursors()` are `async`, and the page does not implement
//! the synchronous [`super::Page`] trait; it has the same methods.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/CursorPage.php

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;

use php_value::{Thrown, Value};

use super::{Cursor, CursorItem};

/// A boxed future the page's closures give.
pub type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = Result<T, Thrown>>>>;

/// `Closure(): int`: runs the COUNT query on demand.
pub type TotalCountProvider = Box<dyn Fn() -> LocalBoxFuture<i64>>;

/// `Closure(mixed, bool): Cursor`: builds a Cursor pointing at a given row.
pub type CursorForItem = Box<dyn Fn(Value, bool) -> LocalBoxFuture<Cursor>>;

/// Immutable result of a single cursor pagination — the cursor-side counterpart
/// to WindowPage.
///
/// It is returned by CursorPaginator::paginate() instead of the paginator
/// itself, so the result is not stateful and there is no temporal coupling
/// between method calls: every accessor is safe to call in any order.
pub struct CursorPage {
    items: Vec<Value>,
    has_more: bool,
    cursor: Option<Cursor>,
    total_count_provider: TotalCountProvider,
    cursor_for_item: CursorForItem,
    /// Memoizes the COUNT query, which is only run when asked for.
    total_count: Cell<Option<i64>>,
}

impl std::fmt::Debug for CursorPage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CursorPage")
            .field("items", &self.items)
            .field("has_more", &self.has_more)
            .field("cursor", &self.cursor)
            .field("total_count", &self.total_count.get())
            .finish_non_exhaustive()
    }
}

fn logic_exception(message: &str) -> Thrown {
    Thrown::new("LogicException", message, 0)
}

impl CursorPage {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\CursorPage";

    /// `new CursorPage($items, $hasMore, $cursor, $totalCountProvider, $cursorForItem)`.
    pub fn new(
        items: Vec<Value>,
        has_more: bool,
        cursor: Option<Cursor>,
        total_count_provider: TotalCountProvider,
        cursor_for_item: CursorForItem,
    ) -> Self {
        Self {
            items,
            has_more,
            cursor,
            total_count_provider,
            cursor_for_item,
            total_count: Cell::new(None),
        }
    }

    /// Returns the raw entities on this page.
    pub fn get_items(&self) -> &[Value] {
        &self.items
    }

    /// `getIterator()`: the items, in order.
    pub fn get_iterator(&self) -> std::slice::Iter<'_, Value> {
        self.items.iter()
    }

    /// Returns the number of items on this page.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Executes an extra COUNT query and returns the total number of matching
    /// root entities, ignoring the cursor and limit. The result is memoized.
    pub async fn get_total_count(&self) -> Result<i64, Thrown> {
        if let Some(total_count) = self.total_count.get() {
            return Ok(total_count);
        }
        let total_count = (self.total_count_provider)().await?;
        self.total_count.set(Some(total_count));
        Ok(total_count)
    }

    /// Returns whether there is a previous page.
    pub fn has_previous_page(&self) -> bool {
        self.cursor
            .as_ref()
            .is_some_and(|cursor| cursor.is_next() || self.has_more)
    }

    /// Returns whether there is a next page.
    pub fn has_next_page(&self) -> bool {
        self.has_more || self.cursor.as_ref().is_some_and(Cursor::is_previous)
    }

    /// Returns whether the result set spans more than one page.
    pub fn has_to_paginate(&self) -> bool {
        self.has_previous_page() || self.has_next_page()
    }

    /// A `LogicException` if there is no next page. Check has_next_page() first.
    pub async fn get_next_cursor(&self) -> Result<Cursor, Thrown> {
        match self.items.last() {
            Some(last) if self.has_next_page() => self.get_cursor_for_item(last, true).await,
            _ => Err(logic_exception(
                "There is no next page. Call hasNextPage() before getNextCursor().",
            )),
        }
    }

    /// A `LogicException` if there is no previous page. Check has_previous_page() first.
    pub async fn get_previous_cursor(&self) -> Result<Cursor, Thrown> {
        match self.items.first() {
            Some(first) if self.has_previous_page() => self.get_cursor_for_item(first, false).await,
            _ => Err(logic_exception(
                "There is no previous page. Call hasPreviousPage() before getPreviousCursor().",
            )),
        }
    }

    /// Returns the encoded cursor string for the next page.
    pub async fn get_next_cursor_as_string(&self) -> Result<String, Thrown> {
        Ok(self.get_next_cursor().await?.encode_to_string())
    }

    /// Returns the encoded cursor string for the previous page.
    pub async fn get_previous_cursor_as_string(&self) -> Result<String, Thrown> {
        Ok(self.get_previous_cursor().await?.encode_to_string())
    }

    /// Builds a Cursor pointing at a specific entity (`$isNext` is `true`
    /// by default).
    pub async fn get_cursor_for_item(&self, item: &Value, is_next: bool) -> Result<Cursor, Thrown> {
        (self.cursor_for_item)(item.clone(), is_next).await
    }

    /// Returns the items wrapped with their associated cursors.
    pub async fn get_items_with_cursors(&self) -> Result<Vec<CursorItem>, Thrown> {
        let mut items = Vec::with_capacity(self.items.len());
        for item in &self.items {
            items.push(CursorItem::new(
                item.clone(),
                self.get_cursor_for_item(item, true).await?,
            ));
        }
        Ok(items)
    }
}
