//! Port of tests/Mapping/Driver/FileClassLocatorTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6, over a
//! copy of the fixture files in `tests/data/Mapping` (`__DIR__ . '/..'`
//! is that directory). The fixture classes are declared with their files
//! (`ClassInfo::file`), which PHP learns by including them.
//!
//! Upstream: doctrine/persistence tests/Mapping/Driver/FileClassLocatorTest.php

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]
#![allow(
    clippy::doc_markdown,
    clippy::disallowed_names,
    reason = "upstream's test, class and field names"
)]
#![allow(non_snake_case)]

mod common;

use std::path::PathBuf;

use common::{ENTITY, ENTITY_FIXTURE, FOO, TEST_CLASS, assert_thrown, mapping_dir};
use php2rust_doctrine_persistence::mapping::driver::{ClassLocator, FileClassLocator};
use php2rust_doctrine_persistence::mapping::mapping_exception;

fn colocated() -> String {
    common::register_fixtures();
    format!("{}/_files/colocated", mapping_dir())
}

fn sorted(mut list: Vec<String>) -> Vec<String> {
    list.sort();
    list
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn testGetClassNames() {
    let dir = colocated();
    let locator = FileClassLocator::new([
        PathBuf::from(format!("{dir}/Entity.php")),
        PathBuf::from(format!("{dir}/EntityFixture.php")),
    ]);

    let classes = sorted(locator.get_class_names().unwrap());

    assert_eq!(names(&[ENTITY, ENTITY_FIXTURE]), classes);
}

#[test]
fn testGetClassNamesWithEmptyIterator() {
    common::register_fixtures();
    let locator = FileClassLocator::new(Vec::<PathBuf>::new());
    assert!(locator.get_class_names().unwrap().is_empty());
}

#[test]
fn testCreateFromDirectory() {
    let locator = FileClassLocator::create_from_directories(&[colocated()], &[], ".php").unwrap();

    let classes = sorted(locator.get_class_names().unwrap());

    assert_eq!(names(&[ENTITY, ENTITY_FIXTURE, TEST_CLASS]), classes);
}

#[test]
fn testCreateFromDirectoryWithExtension() {
    let locator = FileClassLocator::create_from_directories(&[colocated()], &[], ".mphp").unwrap();

    let classes = sorted(locator.get_class_names().unwrap());

    assert_eq!(names(&[FOO]), classes);
}

#[test]
fn testCreateFromDirectoryWithNonExistentDirectory() {
    assert_thrown(
        FileClassLocator::create_from_directories(
            &["/non/existent/directory".to_owned()],
            &[],
            ".php",
        ),
        mapping_exception::CLASS,
        "File mapping drivers must have a valid directory path, however the given path [/non/existent/directory] seems to be incorrect!",
    );
}

#[test]
fn testCreateFromEmptyDirectory() {
    common::register_fixtures();
    let locator = FileClassLocator::create_from_directories(
        &[format!("{}/_files/Bar", mapping_dir())],
        &[],
        ".php",
    )
    .unwrap();

    assert!(locator.get_class_names().unwrap().is_empty());
}

#[test]
fn testCreateFromDirectoryIterator() {
    let dir = colocated();
    // new DirectoryIterator($dir): the dot entries, then the directory's entries.
    let mut files = vec![
        PathBuf::from(format!("{dir}/.")),
        PathBuf::from(format!("{dir}/..")),
    ];
    files.extend(
        std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().path()),
    );
    let locator = FileClassLocator::new(files);

    let classes = sorted(locator.get_class_names().unwrap());

    assert_eq!(names(&[ENTITY, ENTITY_FIXTURE, FOO, TEST_CLASS]), classes);
}
