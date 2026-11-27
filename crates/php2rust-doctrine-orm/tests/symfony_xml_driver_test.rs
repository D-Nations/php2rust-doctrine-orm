//! Port of the Symfony `XmlDriverTest` and the `DriverTestCase` tests it
//! inherits. Each test has its own directory under `sys_get_temp_dir()`
//! (upstream's tests run one at a time and share
//! `abstract_driver_test`), which `tearDown()` removes.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Mapping/Symfony/XmlDriverTest.php tests/Tests/ORM/Mapping/Symfony/DriverTestCase.php

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::fs;
use std::path::PathBuf;

use php2rust_doctrine_orm::mapping::driver::XmlDriver;
use php2rust_doctrine_orm::mapping::driver::xml_driver::SIMPLIFIED_DEFAULT_FILE_EXTENSION;
use php2rust_doctrine_persistence::mapping::driver::FileDriver;
use php2rust_doctrine_persistence::php_reflection::thrown_is_a;

/// `getFileExtension()`.
const FILE_EXTENSION: &str = ".orm.xml";

/// The test's directory, removed when dropped (`tearDown()`).
struct Dir(PathBuf);

impl Dir {
    /// `setUp()`.
    fn set_up(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("abstract_driver_test_{test}"));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }

    /// `touch($filename = $this->dir . $name)`.
    fn touch(&self, name: &str) -> String {
        let filename = format!("{}/{name}", self.path());
        fs::write(&filename, b"").unwrap();
        filename
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// `getDriver($paths)`: `new SimplifiedXmlDriver(array_flip($paths))`.
fn get_driver(paths: &[(&str, &str)]) -> FileDriver<XmlDriver> {
    let prefixes: Vec<(String, String)> = paths
        .iter()
        .map(|(namespace, path)| ((*path).to_owned(), (*namespace).to_owned()))
        .collect();
    XmlDriver::simplified(&prefixes, SIMPLIFIED_DEFAULT_FILE_EXTENSION, true).unwrap()
}

fn expect_mapping_exception(dir: &Dir, class_name: &str) -> String {
    let driver = get_driver(&[("MyNamespace\\MySubnamespace\\Entity", &dir.path())]);

    let Err(e) = driver.get_locator().find_mapping_file(class_name) else {
        panic!("expected a MappingException");
    };
    assert!(thrown_is_a(
        &e,
        "Doctrine\\Persistence\\Mapping\\MappingException"
    ));
    String::from_utf8_lossy(e.message.as_bytes()).into_owned()
}

#[test]
fn testFindMappingFile() {
    let dir = Dir::set_up("find_mapping_file");
    let driver = get_driver(&[
        ("MyNamespace\\MySubnamespace\\EntityFoo", "foo"),
        ("MyNamespace\\MySubnamespace\\Entity", &dir.path()),
    ]);

    let filename = dir.touch(&format!("Foo{FILE_EXTENSION}"));
    assert_eq!(
        filename,
        driver
            .get_locator()
            .find_mapping_file("MyNamespace\\MySubnamespace\\Entity\\Foo")
            .unwrap()
    );
}

#[test]
fn testFindMappingFileInSubnamespace() {
    let dir = Dir::set_up("find_mapping_file_in_subnamespace");
    let driver = get_driver(&[("MyNamespace\\MySubnamespace\\Entity", &dir.path())]);

    let filename = dir.touch(&format!("Foo.Bar{FILE_EXTENSION}"));
    assert_eq!(
        filename,
        driver
            .get_locator()
            .find_mapping_file("MyNamespace\\MySubnamespace\\Entity\\Foo\\Bar")
            .unwrap()
    );
}

#[test]
fn testFindMappingFileNamespacedFoundFileNotFound() {
    let dir = Dir::set_up("namespaced_found_file_not_found");
    let message = expect_mapping_exception(&dir, "MyNamespace\\MySubnamespace\\Entity\\Foo");
    assert!(message.contains("No mapping file found named"), "{message}");
}

#[test]
fn testFindMappingNamespaceNotFound() {
    let dir = Dir::set_up("namespace_not_found");
    let message = expect_mapping_exception(&dir, "MyOtherNamespace\\MySubnamespace\\Entity\\Foo");
    assert!(
        message.contains(&format!(
            "No mapping file found named 'Foo{FILE_EXTENSION}' for class 'MyOtherNamespace\\MySubnamespace\\Entity\\Foo'."
        )),
        "{message}"
    );
}
