//! Port of tests/Mapping/FileDriverTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6. `TestFileDriver` is a
//! `FileDriver` over `TestFileDriverImpl`; the mocked locator is a
//! `MockFileLocator`.
//!
//! Upstream: doctrine/persistence tests/Mapping/FileDriverTest.php

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

use std::sync::{Arc, Mutex};

use common::{
    ANOTHER_GLOBAL_CLASS, GLOBAL_CLASS, NOT_LOADED_CLASS, TestClassMetadata, mapping_dir,
};
use php_value::Thrown;
use php2rust_doctrine_persistence::mapping::ClassMetadata;
use php2rust_doctrine_persistence::mapping::driver::{
    FileDriver, FileDriverImpl, FileLocator, MappingDriver,
};

const STD_CLASS: &str = "stdClass";

/// `TestFileDriver`'s `loadMappingFile()` and `loadMetadataForClass()`.
struct TestFileDriverImpl;

impl FileDriverImpl for TestFileDriverImpl {
    type Element = TestClassMetadata;

    fn load_mapping_file(&self, file: &str) -> Result<Vec<(String, TestClassMetadata)>, Thrown> {
        if file.contains("global.yml") {
            return Ok(vec![
                (
                    GLOBAL_CLASS.to_owned(),
                    TestClassMetadata::new(GLOBAL_CLASS),
                ),
                (
                    ANOTHER_GLOBAL_CLASS.to_owned(),
                    TestClassMetadata::new(ANOTHER_GLOBAL_CLASS),
                ),
            ]);
        }

        Ok(vec![(
            STD_CLASS.to_owned(),
            TestClassMetadata::new(STD_CLASS),
        )])
    }

    fn load_metadata_for_class(
        &self,
        _driver: &FileDriver<Self>,
        _class_name: &str,
        _metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        Ok(())
    }
}

type TestFileDriver = FileDriver<TestFileDriverImpl>;

/// `$this->createMock(FileLocator::class)` (or a stub): `.yml` files in
/// `_files`, with the configured answers, counting the calls.
#[derive(Default)]
struct MockFileLocator {
    find_mapping_file: Option<(String, String)>,
    all_class_names: Vec<String>,
    file_exists: Option<(String, bool)>,
    find_mapping_file_calls: Mutex<Vec<String>>,
    get_all_class_names_calls: Mutex<Vec<String>>,
    file_exists_calls: Mutex<Vec<String>>,
}

impl FileLocator for MockFileLocator {
    fn find_mapping_file(&self, class_name: &str) -> Result<String, Thrown> {
        self.find_mapping_file_calls
            .lock()
            .unwrap()
            .push(class_name.to_owned());
        Ok(self
            .find_mapping_file
            .as_ref()
            .filter(|(name, _)| name == class_name)
            .map_or_default(|(_, file)| file.clone()))
    }

    fn get_all_class_names(&self, global_basename: &str) -> Result<Vec<String>, Thrown> {
        self.get_all_class_names_calls
            .lock()
            .unwrap()
            .push(global_basename.to_owned());
        Ok(self.all_class_names.clone())
    }

    fn file_exists(&self, class_name: &str) -> bool {
        self.file_exists_calls
            .lock()
            .unwrap()
            .push(class_name.to_owned());
        self.file_exists
            .as_ref()
            .is_some_and(|(name, exists)| name == class_name && *exists)
    }

    fn get_paths(&self) -> Vec<String> {
        vec![format!("{}/_files", mapping_dir())]
    }

    fn get_file_extension(&self) -> Option<String> {
        Some(".yml".to_owned())
    }
}

fn std_class_file() -> String {
    format!("{}/_files/stdClass.yml", mapping_dir())
}

fn new_locator() -> MockFileLocator {
    common::register_fixtures();
    MockFileLocator::default()
}

fn create_test_file_driver(locator: Arc<MockFileLocator>) -> TestFileDriver {
    let locator: Arc<dyn FileLocator> = locator;
    FileDriver::new(TestFileDriverImpl, locator, None)
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn testGlobalBasename() {
    let driver: TestFileDriver = FileDriver::new(TestFileDriverImpl, Vec::<String>::new(), None);

    assert_eq!("", driver.get_global_basename());

    driver.set_global_basename("global");

    assert_eq!("global", driver.get_global_basename());
}

#[test]
fn testGetElementFromGlobalFile() {
    let driver = create_test_file_driver(Arc::new(new_locator()));

    driver.set_global_basename("global");

    let element = driver.get_element(GLOBAL_CLASS).unwrap();

    assert_eq!(GLOBAL_CLASS, element.get_name());
}

#[test]
fn testGetElementFromFile() {
    let locator = Arc::new(MockFileLocator {
        find_mapping_file: Some((STD_CLASS.to_owned(), std_class_file())),
        ..new_locator()
    });

    let driver = create_test_file_driver(Arc::clone(&locator));

    assert_eq!(STD_CLASS, driver.get_element(STD_CLASS).unwrap().get_name());
    assert_eq!(
        *locator.find_mapping_file_calls.lock().unwrap(),
        [STD_CLASS]
    );
}

#[test]
fn testGetElementUpdatesClassCache() {
    let locator = Arc::new(MockFileLocator {
        find_mapping_file: Some((STD_CLASS.to_owned(), std_class_file())),
        ..new_locator()
    });

    let driver = create_test_file_driver(Arc::clone(&locator));

    // not cached
    assert_eq!(STD_CLASS, driver.get_element(STD_CLASS).unwrap().get_name());

    // cached call
    assert_eq!(STD_CLASS, driver.get_element(STD_CLASS).unwrap().get_name());

    // findMappingFile should only be called once
    assert_eq!(
        *locator.find_mapping_file_calls.lock().unwrap(),
        [STD_CLASS]
    );
}

#[test]
fn testGetAllClassNamesGlobalBasename() {
    let locator = Arc::new(MockFileLocator {
        all_class_names: names(&[GLOBAL_CLASS, ANOTHER_GLOBAL_CLASS]),
        ..new_locator()
    });

    let driver = create_test_file_driver(Arc::clone(&locator));
    driver.set_global_basename("global");

    let class_names = driver.get_all_class_names().unwrap();

    assert_eq!(names(&[GLOBAL_CLASS, ANOTHER_GLOBAL_CLASS]), class_names);
    assert!(
        locator
            .get_all_class_names_calls
            .lock()
            .unwrap()
            .iter()
            .all(|basename| basename == "global")
    );
}

#[test]
fn testGetAllClassNamesFromMappingFile() {
    let locator = Arc::new(MockFileLocator {
        all_class_names: names(&[STD_CLASS]),
        ..new_locator()
    });
    let driver = create_test_file_driver(Arc::clone(&locator));

    let class_names = driver.get_all_class_names().unwrap();

    assert_eq!(names(&[STD_CLASS]), class_names);
    // ->with(self::equalTo(null)): the global basename is '', which equals null
    assert!(
        locator
            .get_all_class_names_calls
            .lock()
            .unwrap()
            .iter()
            .all(String::is_empty)
    );
}

#[test]
fn testGetAllClassNamesBothSources() {
    let locator = Arc::new(MockFileLocator {
        all_class_names: names(&[STD_CLASS]),
        ..new_locator()
    });
    let driver = create_test_file_driver(Arc::clone(&locator));
    driver.set_global_basename("global");

    let class_names = driver.get_all_class_names().unwrap();

    assert_eq!(
        names(&[GLOBAL_CLASS, ANOTHER_GLOBAL_CLASS, STD_CLASS]),
        class_names
    );
}

#[test]
fn testGetAllClassNamesBothSourcesNoDupes() {
    let locator = Arc::new(MockFileLocator {
        all_class_names: names(&[STD_CLASS]),
        find_mapping_file: Some((STD_CLASS.to_owned(), std_class_file())),
        ..new_locator()
    });
    let driver = create_test_file_driver(Arc::clone(&locator));
    driver.set_global_basename("global");

    driver.get_element(STD_CLASS).unwrap();
    let class_names = driver.get_all_class_names().unwrap();

    assert_eq!(
        names(&[GLOBAL_CLASS, ANOTHER_GLOBAL_CLASS, STD_CLASS]),
        class_names
    );
    assert_eq!(
        *locator.get_all_class_names_calls.lock().unwrap(),
        ["global"]
    );
    assert_eq!(
        *locator.find_mapping_file_calls.lock().unwrap(),
        [STD_CLASS]
    );
}

#[test]
fn testIsNotTransient() {
    let locator = Arc::new(MockFileLocator {
        file_exists: Some((STD_CLASS.to_owned(), true)),
        ..new_locator()
    });

    let driver = create_test_file_driver(Arc::clone(&locator));
    driver.set_global_basename("global");

    assert!(!driver.is_transient(STD_CLASS).unwrap());
    assert!(!driver.is_transient(GLOBAL_CLASS).unwrap());
    assert!(!driver.is_transient(ANOTHER_GLOBAL_CLASS).unwrap());
    assert_eq!(*locator.file_exists_calls.lock().unwrap(), [STD_CLASS]);
}

#[test]
fn testIsTransient() {
    let locator = Arc::new(MockFileLocator {
        file_exists: Some((NOT_LOADED_CLASS.to_owned(), false)),
        ..new_locator()
    });

    let driver = create_test_file_driver(Arc::clone(&locator));

    assert!(driver.is_transient(NOT_LOADED_CLASS).unwrap());
    assert_eq!(
        *locator.file_exists_calls.lock().unwrap(),
        [NOT_LOADED_CLASS]
    );
}

#[test]
fn testNonLocatorFallback() {
    common::register_fixtures();
    let driver: TestFileDriver = FileDriver::new(
        TestFileDriverImpl,
        format!("{}/_files", mapping_dir()).as_str(),
        Some(".yml"),
    );
    assert!(driver.is_transient(NOT_LOADED_CLASS).unwrap());
    assert!(!driver.is_transient(STD_CLASS).unwrap());
}
