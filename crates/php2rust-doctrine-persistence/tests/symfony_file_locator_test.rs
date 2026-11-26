//! Port of tests/Mapping/SymfonyFileLocatorTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6, over a
//! copy of the fixture files in `tests/data/Mapping`.
//!
//! Upstream: doctrine/persistence tests/Mapping/SymfonyFileLocatorTest.php

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
use php2rust_doctrine_persistence::mapping::driver::files::realpath;
use php2rust_doctrine_persistence::mapping::driver::{FileLocator, SymfonyFileLocator};
use php2rust_doctrine_persistence::mapping::mapping_exception;

fn dir() -> String {
    mapping_dir()
}

fn prefixes(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(p, n)| ((*p).to_owned(), (*n).to_owned()))
        .collect()
}

fn sorted(mut list: Vec<String>) -> Vec<String> {
    list.sort();
    list
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn testGetPaths() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(""), ".").unwrap();
    assert_eq!(std::slice::from_ref(&path), &*locator.get_paths());

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(""), ".").unwrap();
    assert_eq!([path], *locator.get_paths());
}

#[test]
fn testGetPrefixes() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(""), ".").unwrap();
    assert_eq!(
        prefixes(&[(&path, prefix)]),
        locator.get_namespace_prefixes()
    );
}

#[test]
fn testGetFileExtension() {
    let mut locator = SymfonyFileLocator::new(Vec::new(), Some(".yml"), ".").unwrap();
    assert_eq!(Some(".yml".to_owned()), locator.get_file_extension());
    locator.set_file_extension(".xml");
    assert_eq!(Some(".xml".to_owned()), locator.get_file_extension());
}

#[test]
fn testFileExists() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), ".").unwrap();

    assert!(locator.file_exists("Foo\\stdClass"));
    assert!(locator.file_exists("Foo\\global"));
    assert!(!locator.file_exists("Foo\\stdClass2"));
    assert!(!locator.file_exists("Foo\\global2"));
}

#[test]
fn testGetAllClassNames() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), ".").unwrap();
    let all_classes = locator.get_all_class_names("").unwrap();
    let global_classes = locator.get_all_class_names("global").unwrap();

    let expected_all_classes = strings(&["Foo\\Bar\\subDirClass", "Foo\\global", "Foo\\stdClass"]);
    let expected_global_classes = strings(&["Foo\\Bar\\subDirClass", "Foo\\stdClass"]);

    assert_eq!(sorted(expected_all_classes), sorted(all_classes));
    assert_eq!(sorted(expected_global_classes), sorted(global_classes));
}

#[test]
fn testInvalidCustomNamespaceSeparator() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    assert_thrown(
        SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), ""),
        "InvalidArgumentException",
        "Namespace separator should not be empty",
    );
}

/// `customNamespaceSeparatorProvider()`.
fn custom_namespace_separator_provider() -> [(&'static str, &'static str); 2] {
    [
        // 'directory separator'
        (DIRECTORY_SEPARATOR, "/_custom_ns/dir"),
        // 'default dot separator'
        (".", "/_custom_ns/dot"),
    ]
}

#[test]
fn testGetClassNamesWithCustomNsSeparator() {
    for (separator, dir_name) in custom_namespace_separator_provider() {
        let path = format!("{}{dir_name}", dir());
        let prefix = "Foo";

        let locator =
            SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), separator).unwrap();
        let classes = sorted(locator.get_all_class_names("").unwrap());

        assert_eq!(
            strings(&[
                "Foo\\stdClass",
                "Foo\\sub\\subClass",
                "Foo\\sub\\subsub\\subSubClass"
            ]),
            classes
        );
    }
}

/// A separator, a directory and its files with the class names they map to.
type LookupCase = (
    &'static str,
    &'static str,
    Vec<(&'static str, &'static str)>,
);

/// `customNamespaceLookupQueryProvider()`.
fn custom_namespace_lookup_query_provider() -> Vec<LookupCase> {
    vec![
        // 'directory separator'
        (
            DIRECTORY_SEPARATOR,
            "/_custom_ns/dir",
            vec![
                ("stdClass.yml", "Foo\\stdClass"),
                ("sub/subClass.yml", "Foo\\sub\\subClass"),
                (
                    "sub/subsub/subSubClass.yml",
                    "Foo\\sub\\subsub\\subSubClass",
                ),
            ],
        ),
        // 'default dot separator'
        (
            ".",
            "/_custom_ns/dot",
            vec![
                ("stdClass.yml", "Foo\\stdClass"),
                ("sub.subClass.yml", "Foo\\sub\\subClass"),
                (
                    "sub.subsub.subSubClass.yml",
                    "Foo\\sub\\subsub\\subSubClass",
                ),
            ],
        ),
    ]
}

#[test]
fn testFindMappingFileWithCustomNsSeparator() {
    for (separator, dir_name, files) in custom_namespace_lookup_query_provider() {
        let path = format!("{}{dir_name}", dir());
        let prefix = "Foo";

        let locator =
            SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), separator).unwrap();

        for (file_path, class_name) in files {
            assert_eq!(
                realpath(&format!("{path}/{file_path}")),
                realpath(&locator.find_mapping_file(class_name).unwrap())
            );
        }
    }
}

#[test]
fn testFindMappingFile() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), ".").unwrap();

    assert_eq!(
        format!("{}/_files/stdClass.yml", dir()),
        locator.find_mapping_file("Foo\\stdClass").unwrap()
    );
}

#[test]
fn testFindMappingFileNotFound() {
    let path = format!("{}/_files", dir());
    let prefix = "Foo";

    let locator = SymfonyFileLocator::new(prefixes(&[(&path, prefix)]), Some(".yml"), ".").unwrap();

    assert_thrown(
        locator.find_mapping_file("Foo\\stdClass2"),
        mapping_exception::CLASS,
        "No mapping file found named 'stdClass2.yml' for class 'Foo\\stdClass2'.",
    );
}

#[test]
fn testFindMappingFileNotFoundForClassInRootNamespace() {
    let locator = SymfonyFileLocator::new(Vec::new(), Some(".yml"), ".").unwrap();

    assert_thrown(
        locator.find_mapping_file("DateTime"),
        mapping_exception::CLASS,
        "No mapping file found named 'DateTime.yml' for class 'DateTime'.",
    );
}

#[test]
fn testFindMappingFileLeastSpecificNamespaceFirst() {
    // Low -> High
    let match_ns = format!("{}/_match_ns", dir());
    let match_ns_bar = format!("{}/_match_ns/Bar", dir());
    let prefixes = prefixes(&[(&match_ns, "Foo"), (&match_ns_bar, "Foo\\Bar")]);

    let locator = SymfonyFileLocator::new(prefixes, Some(".yml"), ".").unwrap();

    assert_eq!(
        format!("{}/_match_ns/Bar/barEntity.yml", dir()),
        locator.find_mapping_file("Foo\\Bar\\barEntity").unwrap()
    );
}

#[test]
fn testFindMappingFileMostSpecificNamespaceFirst() {
    let match_ns = format!("{}/_match_ns", dir());
    let match_ns_bar = format!("{}/_match_ns/Bar", dir());
    let prefixes = prefixes(&[(&match_ns_bar, "Foo\\Bar"), (&match_ns, "Foo")]);

    let locator = SymfonyFileLocator::new(prefixes, Some(".yml"), ".").unwrap();

    assert_eq!(
        format!("{}/_match_ns/Bar/barEntity.yml", dir()),
        locator.find_mapping_file("Foo\\Bar\\barEntity").unwrap()
    );
}
