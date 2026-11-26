//! `Page` and `WindowPage`: the immutable result of a pagination.
//!
//! Upstream: doctrine/orm src/Tools/Pagination/Page.php src/Tools/Pagination/WindowPage.php

use php_value::{Thrown, Value};

use super::Window;

/// Immutable result of a single pagination.
///
/// A page is the return value of PaginatorInterface::paginate(). It is
/// iterable and countable, and carries no temporal coupling: every accessor is
/// safe to call in any order, and paginating again never affects a page you
/// already hold.
///
/// Navigating to another page is deliberately left out of this interface: the
/// position types are strategy specific (WindowPage::getNextWindow()
/// returns a Window, CursorPage::getNextCursor() a Cursor).
pub trait Page {
    /// Returns the raw entities on this page.
    fn get_items(&self) -> &[Value];

    /// `getIterator()`: the items, in order.
    fn get_iterator(&self) -> std::slice::Iter<'_, Value> {
        self.get_items().iter()
    }

    /// Returns the number of items on this page.
    fn count(&self) -> usize;

    /// Returns the total number of matching root entities, ignoring the
    /// pagination position.
    fn get_total_count(&self) -> i64;

    /// Returns whether there is a previous page.
    fn has_previous_page(&self) -> bool;

    /// Returns whether there is a next page.
    fn has_next_page(&self) -> bool;

    /// Returns whether the result set spans more than one page.
    fn has_to_paginate(&self) -> bool;
}

/// Immutable result of a single offset pagination.
///
/// It is returned by OffsetPaginator::paginate() instead of the paginator
/// itself, so the result is not stateful and there is no temporal coupling
/// between method calls: every accessor is safe to call in any order.
///
/// On top of Page, a window page exposes what only offset pagination can
/// know: the total number of pages and the position of the last one.
#[derive(Debug, Clone)]
pub struct WindowPage {
    items: Vec<Value>,
    total_count: i64,
    window: Window,
}

fn len(items: &[Value]) -> i64 {
    i64::try_from(items.len()).unwrap_or(i64::MAX)
}

impl WindowPage {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Tools\\Pagination\\WindowPage";

    /// `new WindowPage($items, $totalCount, $window)`.
    pub fn new(items: Vec<Value>, total_count: i64, window: Window) -> Self {
        Self {
            items,
            total_count,
            window,
        }
    }

    /// Returns the window that produced this page.
    pub fn get_window(&self) -> Window {
        self.window
    }

    /// Returns the 1-based number of this page.
    pub fn get_page_number(&self) -> i64 {
        self.window.get_page_number()
    }

    /// Returns the total number of pages, at least 1 even for an empty result
    /// set.
    pub fn get_page_count(&self) -> i64 {
        let max_results = self.window.get_max_results();

        1.max((self.total_count + max_results - 1) / max_results)
    }

    /// Returns the window for the next page (a `LogicException` if there is
    /// no next page). Check has_next_page() first.
    pub fn get_next_window(&self) -> Result<Window, Thrown> {
        if !self.has_next_page() {
            return Err(Thrown::new(
                "LogicException",
                "There is no next page. Call hasNextPage() before getNextWindow().",
                0,
            ));
        }

        self.window.next()
    }

    /// Returns the window for the previous page (a `LogicException` if there
    /// is no previous page). Check has_previous_page() first.
    pub fn get_previous_window(&self) -> Result<Window, Thrown> {
        if !self.has_previous_page() {
            return Err(Thrown::new(
                "LogicException",
                "There is no previous page. Call hasPreviousPage() before getPreviousWindow().",
                0,
            ));
        }

        self.window.previous()
    }

    /// Returns the window of the last page, keeping the same page size.
    ///
    /// Unlike get_next_window() and get_previous_window(), this never
    /// throws: an empty result set has a single, empty first page.
    pub fn get_last_window(&self) -> Result<Window, Thrown> {
        Window::from_page_number_and_size(self.get_page_count(), self.window.get_max_results())
    }
}

impl Page for WindowPage {
    fn get_items(&self) -> &[Value] {
        &self.items
    }

    fn count(&self) -> usize {
        self.items.len()
    }

    fn get_total_count(&self) -> i64 {
        self.total_count
    }

    fn has_previous_page(&self) -> bool {
        self.window.get_first_result() > 0
    }

    fn has_next_page(&self) -> bool {
        self.window.get_first_result() + len(&self.items) < self.total_count
    }

    fn has_to_paginate(&self) -> bool {
        self.total_count > len(&self.items)
    }
}
