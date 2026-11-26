//! WindowTest.php at doctrine/orm 00c2fd70da291cca1ec75f4dae5344fe3df588fa.
//! `testWindowIsFinal` is n/a: a Rust struct has no subclasses.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Pagination/WindowTest.php

#![allow(clippy::unwrap_used, reason = "a panic is a test failure")]
#![allow(non_snake_case)]

use php2rust_doctrine_orm::tools::pagination::Window;

#[test]
fn testGetters() {
    let window = Window::new(50, 25).unwrap();

    assert_eq!(50, window.get_first_result());
    assert_eq!(25, window.get_max_results());
}

#[test]
fn testNext() {
    let next = Window::new(0, 25).unwrap().next().unwrap();

    assert_eq!(25, next.get_first_result());
    assert_eq!(25, next.get_max_results());
}

#[test]
fn testPrevious() {
    let previous = Window::new(50, 25).unwrap().previous().unwrap();

    assert_eq!(25, previous.get_first_result());
    assert_eq!(25, previous.get_max_results());
}

#[test]
fn testPreviousIsClampedAtZero() {
    let previous = Window::new(10, 25).unwrap().previous().unwrap();

    assert_eq!(0, previous.get_first_result());
}

#[test]
fn testFromPageNumberAndSize() {
    let window = Window::from_page_number_and_size(3, 25).unwrap();

    assert_eq!(50, window.get_first_result());
    assert_eq!(25, window.get_max_results());
    assert_eq!(3, window.get_page_number());
}

#[test]
fn testGetPageNumber() {
    assert_eq!(1, Window::new(0, 25).unwrap().get_page_number());
    assert_eq!(2, Window::new(25, 25).unwrap().get_page_number());
    assert_eq!(3, Window::new(50, 25).unwrap().get_page_number());
}

#[test]
fn testNegativeFirstResultThrows() {
    let e = Window::new(-1, 20).unwrap_err();
    assert_eq!("InvalidArgumentException", e.class);
}

#[test]
fn testZeroMaxResultsThrows() {
    let e = Window::new(0, 0).unwrap_err();
    assert_eq!("InvalidArgumentException", e.class);
}

#[test]
fn testFromPageNumberAndSizeWithPageNumberBelowOneThrows() {
    let e = Window::from_page_number_and_size(0, 25).unwrap_err();
    assert_eq!("InvalidArgumentException", e.class);
}

#[test]
fn testIsImmutable() {
    let window = Window::new(0, 25).unwrap();
    let _ = window.next();

    assert_eq!(0, window.get_first_result());
}
