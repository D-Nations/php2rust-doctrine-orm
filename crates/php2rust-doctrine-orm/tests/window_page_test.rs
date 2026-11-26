//! WindowPageTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! The `(object) ['id' => N]` items are `['id' => N]` arrays; a page's
//! iterator is its items in order.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/WindowPageTest.php

#![allow(clippy::unwrap_used, reason = "a panic is a test failure")]
#![allow(non_snake_case)]

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::tools::pagination::{Page, Window, WindowPage};

fn item(id: i64) -> Value {
    let mut item = PhpArray::new();
    item.insert("id", Value::Int(id));
    Value::Array(item)
}

fn window(first_result: i64, max_results: i64) -> Window {
    Window::new(first_result, max_results).unwrap()
}

#[test]
fn testGetItemsAndCount() {
    let items = vec![item(1), item(2)];
    let page = WindowPage::new(items.clone(), 10, window(0, 2));

    assert!(
        items
            .iter()
            .zip(page.get_items())
            .all(|(a, b)| a.identical(b))
    );
    assert_eq!(2, page.count());
    assert!(
        items
            .iter()
            .zip(page.get_iterator())
            .all(|(a, b)| a.identical(b))
    );
}

#[test]
fn testGetTotalCount() {
    let page = WindowPage::new(vec![], 42, window(0, 20));

    assert_eq!(42, page.get_total_count());
}

#[test]
fn testFirstPageHasNextButNoPreviousPage() {
    let page = WindowPage::new(vec![item(1), item(2)], 10, window(0, 2));

    assert!(!page.has_previous_page());
    assert!(page.has_next_page());
    assert!(page.has_to_paginate());
}

#[test]
fn testMiddlePageHasBothPages() {
    let page = WindowPage::new(vec![item(3), item(4)], 10, window(2, 2));

    assert!(page.has_previous_page());
    assert!(page.has_next_page());
}

#[test]
fn testLastPageHasPreviousButNoNextPage() {
    let page = WindowPage::new(vec![item(9), item(10)], 10, window(8, 2));

    assert!(page.has_previous_page());
    assert!(!page.has_next_page());
}

#[test]
fn testSinglePageHasNoNavigation() {
    let page = WindowPage::new(vec![item(1)], 1, window(0, 20));

    assert!(!page.has_previous_page());
    assert!(!page.has_next_page());
    assert!(!page.has_to_paginate());
}

#[test]
fn testGetNextWindow() {
    let page = WindowPage::new(vec![item(1), item(2)], 10, window(0, 2));

    assert_eq!(2, page.get_next_window().unwrap().get_first_result());
}

#[test]
fn testGetPreviousWindow() {
    let page = WindowPage::new(vec![item(3), item(4)], 10, window(2, 2));

    assert_eq!(0, page.get_previous_window().unwrap().get_first_result());
}

#[test]
fn testGetNextWindowThrowsWhenNoNextPage() {
    let page = WindowPage::new(vec![item(1)], 1, window(0, 20));

    assert_eq!("LogicException", page.get_next_window().unwrap_err().class);
}

#[test]
fn testGetPreviousWindowThrowsWhenNoPreviousPage() {
    let page = WindowPage::new(vec![item(1)], 10, window(0, 2));

    assert_eq!(
        "LogicException",
        page.get_previous_window().unwrap_err().class
    );
}

/// `assertSame($window, ...)`: the window is a value.
#[test]
fn testGetWindow() {
    let window = window(4, 2);
    let page = WindowPage::new(vec![], 10, window);

    assert_eq!(window, page.get_window());
}

#[test]
fn testGetPageNumber() {
    let page = WindowPage::new(vec![], 10, window(4, 2));

    assert_eq!(3, page.get_page_number());
}

#[test]
fn testGetPageCount() {
    assert_eq!(
        5,
        WindowPage::new(vec![], 10, window(0, 2)).get_page_count()
    );
    assert_eq!(
        4,
        WindowPage::new(vec![], 10, window(0, 3)).get_page_count()
    );
    assert_eq!(
        1,
        WindowPage::new(vec![], 10, window(0, 20)).get_page_count()
    );
}

#[test]
fn testGetPageCountIsAtLeastOneOnEmptyResultSet() {
    assert_eq!(
        1,
        WindowPage::new(vec![], 0, window(0, 20)).get_page_count()
    );
}

#[test]
fn testGetLastWindow() {
    let last_window = WindowPage::new(vec![], 10, window(0, 3))
        .get_last_window()
        .unwrap();

    assert_eq!(9, last_window.get_first_result());
    assert_eq!(3, last_window.get_max_results());
    assert_eq!(4, last_window.get_page_number());
}

#[test]
fn testGetLastWindowOnEmptyResultSetPointsAtTheFirstPage() {
    let last_window = WindowPage::new(vec![], 0, window(0, 20))
        .get_last_window()
        .unwrap();

    assert_eq!(0, last_window.get_first_result());
    assert_eq!(1, last_window.get_page_number());
}
