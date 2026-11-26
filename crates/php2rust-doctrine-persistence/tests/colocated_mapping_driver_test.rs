//! Port of tests/Mapping/ColocatedMappingDriverTest.php at
//! doctrine/persistence d8293831c5b99449b2f63eb1e1bbdc202118daa6, over a
//! copy of the fixture files in `tests/data/Mapping`. `MyDriver` holds a
//! `ColocatedMappingDriver`.
//!
//! Upstream: doctrine/persistence tests/Mapping/ColocatedMappingDriverTest.php

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

use std::sync::Arc;

use common::{ENTITY, ENTITY_FIXTURE, TEST_CLASS, mapping_dir};
use php_value::Thrown;
use php2rust_doctrine_persistence::mapping::ClassMetadata;
use php2rust_doctrine_persistence::mapping::driver::{
    ClassLocator, ClassNames, ColocatedMappingDriver, MappingDriver,
};

/// `MyDriver`.
struct MyDriver {
    colocated: ColocatedMappingDriver,
}

enum Paths {
    Paths(Vec<String>),
    Locator(Arc<dyn ClassLocator>),
}

impl MyDriver {
    /// One or multiple paths where mapping classes can be found, or a
    /// class locator.
    fn new(paths: Paths) -> Self {
        let colocated =
            ColocatedMappingDriver::new("Doctrine\\Tests\\Persistence\\Mapping\\MyDriver");
        match paths {
            Paths::Locator(locator) => colocated.set_class_locator(locator),
            Paths::Paths(paths) => colocated.add_paths(&paths),
        }
        Self { colocated }
    }
}

impl MappingDriver for MyDriver {
    fn load_metadata_for_class(
        &self,
        _class_name: &str,
        _metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        Ok(())
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        self.colocated
            .get_all_class_names(|class_name| self.is_transient(class_name))
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        Ok(class_name == TEST_CLASS)
    }
}

fn colocated() -> String {
    common::register_fixtures();
    format!("{}/_files/colocated", mapping_dir())
}

fn create_directory_path_driver(dir_path: String) -> MyDriver {
    MyDriver::new(Paths::Paths(vec![dir_path]))
}

fn create_class_names_driver(classes: &[&str]) -> MyDriver {
    common::register_fixtures();
    MyDriver::new(Paths::Locator(Arc::new(ClassNames::new(
        classes.iter().copied(),
    ))))
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn testAddGetPaths() {
    let driver = create_directory_path_driver(colocated());
    assert_eq!(vec![colocated()], driver.colocated.get_paths());

    driver
        .colocated
        .add_paths(&strings(&["/test/path1", "/test/path2"]));

    assert_eq!(
        vec![
            colocated(),
            "/test/path1".to_owned(),
            "/test/path2".to_owned()
        ],
        driver.colocated.get_paths()
    );
}

#[test]
fn testAddGetExcludePaths() {
    let driver = create_directory_path_driver(colocated());
    assert!(driver.colocated.get_exclude_paths().is_empty());

    driver
        .colocated
        .add_exclude_paths(&strings(&["/test/path1", "/test/path2"]));

    assert_eq!(
        strings(&["/test/path1", "/test/path2"]),
        driver.colocated.get_exclude_paths()
    );
}

#[test]
fn testGetSetFileExtension() {
    let driver = create_directory_path_driver(colocated());
    assert_eq!(".php", driver.colocated.get_file_extension());

    driver.colocated.set_file_extension(".php1");

    assert_eq!(".php1", driver.colocated.get_file_extension());
}

/// `directoryPathProvider()`.
fn directory_path_provider() -> [String; 2] {
    [
        // 'straigthforward path'
        colocated(),
        // 'winding path'
        format!("{}/../Mapping/_files/colocated", mapping_dir()),
    ]
}

#[test]
fn testGetAllClassNamesForDirectory() {
    for dir_path in directory_path_provider() {
        let driver = create_directory_path_driver(dir_path);

        let mut classes = driver.get_all_class_names().unwrap();

        classes.sort();
        assert_eq!(strings(&[ENTITY, ENTITY_FIXTURE]), classes);
    }
}

#[test]
fn testGetAllClassNamesRemovesTransient() {
    let driver = create_class_names_driver(&[
        // This class is transient, so it should not be returned by getAllClassNames()
        // placed before the Entity class to validate that the driver returns an
        // array without gaps in the indexes
        TEST_CLASS, ENTITY,
    ]);

    let classes = driver.get_all_class_names().unwrap();

    assert_eq!(
        strings(&[ENTITY]),
        classes,
        "The driver should only return the class names for the provided file path names, excluding transient class names."
    );
}

#[test]
fn testGetAllClassNamesWorksBothForFilePathsAndRetroactivelyAddedDirectoryPaths() {
    let driver = create_class_names_driver(&[ENTITY]);

    driver
        .colocated
        .add_paths(&[format!("{}/_files/colocated/", mapping_dir())]);

    let mut classes = driver.get_all_class_names().unwrap();
    classes.sort();

    assert_eq!(
        strings(&[ENTITY, ENTITY_FIXTURE]),
        classes,
        "The driver should return class names from both the provided file path names and the retroactively added directory paths (these should not be ignored)."
    );
}
