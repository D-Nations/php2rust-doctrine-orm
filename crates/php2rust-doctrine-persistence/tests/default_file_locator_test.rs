//! Port of tests/Mapping/DefaultFileLocatorTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6, over a
//! copy of the fixture files in `tests/data/Mapping`.
//!
//! Upstream: doctrine/persistence tests/Mapping/DefaultFileLocatorTest.php

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

use std::path::MAIN_SEPARATOR_STR as DIRECTORY_SEPARATOR;

use common::{assert_thrown, mapping_dir};
use php2rust_doctrine_persistence::mapping::driver::{DefaultFileLocator, FileLocator};
use php2rust_doctrine_persistence::mapping::mapping_exception;

fn files() -> String {
    format!("{}/_files", mapping_dir())
}

fn sorted(mut list: Vec<String>) -> Vec<String> {
    list.sort();
    list
}

#[test]
fn testGetPaths() {
    let path = files();

    let locator = DefaultFileLocator::new([path.clone()], None);
    assert_eq!(std::slice::from_ref(&path), &*locator.get_paths());

    let locator = DefaultFileLocator::new([path.clone()], None);
    assert_eq!([path], *locator.get_paths());
}

#[test]
fn testGetFileExtension() {
    let mut locator = DefaultFileLocator::new(Vec::<String>::new(), Some(".yml"));
    assert_eq!(Some(".yml".to_owned()), locator.get_file_extension());
    locator.set_file_extension(Some(".xml"));
    assert_eq!(Some(".xml".to_owned()), locator.get_file_extension());
}

#[test]
fn testUniquePaths() {
    let path = files();

    let locator = DefaultFileLocator::new([path.clone(), path.clone()], None);
    assert_eq!([path], *locator.get_paths());
}

#[test]
fn testFindMappingFile() {
    let path = files();

    let locator = DefaultFileLocator::new([path], Some(".yml"));

    assert_eq!(
        format!("{}{DIRECTORY_SEPARATOR}stdClass.yml", files()),
        locator.find_mapping_file("stdClass").unwrap()
    );
}

#[test]
fn testFindMappingFileNotFound() {
    let path = files();

    let locator = DefaultFileLocator::new([path], Some(".yml"));

    assert_thrown(
        locator.find_mapping_file("stdClass2"),
        mapping_exception::CLASS,
        "No mapping file found named 'stdClass2.yml' for class 'stdClass2'",
    );
}

#[test]
fn testGetAllClassNames() {
    let path = files();

    let locator = DefaultFileLocator::new([path], Some(".yml"));
    let all_classes = locator.get_all_class_names("").unwrap();
    let global_classes = locator.get_all_class_names("global").unwrap();

    let expected_all_classes = vec![
        "global".to_owned(),
        "stdClass".to_owned(),
        "subDirClass".to_owned(),
    ];
    let expected_global_classes = vec!["subDirClass".to_owned(), "stdClass".to_owned()];

    assert_eq!(sorted(expected_all_classes), sorted(all_classes));
    assert_eq!(sorted(expected_global_classes), sorted(global_classes));
}

#[test]
fn testGetAllClassNamesNonMatchingFileExtension() {
    let path = files();

    let locator = DefaultFileLocator::new([path], Some(".xml"));
    assert_eq!(
        locator.get_all_class_names("global").unwrap(),
        [] as [std::string::String; 0]
    );
}

#[test]
fn testFileExists() {
    let path = files();

    let locator = DefaultFileLocator::new([path], Some(".yml"));

    assert!(locator.file_exists("stdClass"));
    assert!(!locator.file_exists("stdClass2"));
    assert!(locator.file_exists("global"));
    assert!(!locator.file_exists("global2"));
}
