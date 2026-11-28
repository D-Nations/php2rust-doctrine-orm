//! Port of tests/Mapping/DriverChainTest.php at doctrine/persistence
//! d8293831c5b99449b2f63eb1e1bbdc202118daa6. The mocked drivers are
//! `MockMappingDriver`s.
//!
//! Upstream: doctrine/persistence tests/Mapping/DriverChainTest.php

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
use std::sync::atomic::Ordering;

use common::{MANAGER, MODEL, MockMappingDriver, StubClassMetadata, assert_thrown, same};
use php2rust_doctrine_persistence::mapping::driver::{MappingDriver, MappingDriverChain};
use php2rust_doctrine_persistence::mapping::mapping_exception;
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

const DRIVER_CHAIN_ENTITY: &str = "Doctrine\\Tests\\Persistence\\Mapping\\DriverChainEntity";

fn register() {
    common::register_fixtures();
    register_class(ClassInfo::class(DRIVER_CHAIN_ENTITY));
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn testDelegateToMatchingNamespaceDriver() {
    // #[TestWith] data sets
    for (namespace1, namespace2) in [
        (
            "Doctrine\\Tests\\Models\\Company",
            "Doctrine\\Tests\\Persistence\\Mapping",
        ),
        (
            "Doctrine\\Tests\\Persistence\\Map\\",
            "Doctrine\\Tests\\Persistence\\Map",
        ),
    ] {
        register();
        let class_name = DRIVER_CHAIN_ENTITY;
        let class_metadata = StubClassMetadata::new();

        let chain = MappingDriverChain::new();

        let driver1 = MockMappingDriver::stub();
        let driver2 = Arc::new(MockMappingDriver {
            is_transient: Some(Box::new(|_| true)),
            ..MockMappingDriver::default()
        });

        chain.add_driver(driver1.clone(), namespace1);
        chain.add_driver(driver2.clone(), namespace2);

        chain
            .load_metadata_for_class(class_name, &*class_metadata)
            .unwrap();

        assert!(chain.is_transient(class_name).unwrap());

        assert_eq!(driver1.load_metadata_calls(), Vec::<String>::new());
        assert_eq!(driver1.is_transient_calls(), Vec::<String>::new());
        assert_eq!(driver2.load_metadata_calls(), [class_name]);
        assert_eq!(driver2.is_transient_calls(), [class_name]);
    }
}

#[test]
fn testLoadMetadataShouldThrowMappingExceptionWhenNoDelegatorWasFound() {
    register();
    let class_name = DRIVER_CHAIN_ENTITY;
    let class_metadata = StubClassMetadata::new();

    let chain = MappingDriverChain::new();

    assert_thrown(
        chain.load_metadata_for_class(class_name, &*class_metadata),
        mapping_exception::CLASS,
        "",
    );
}

#[test]
fn testGatherAllClassNames() {
    let chain = MappingDriverChain::new();

    let driver1 = Arc::new(MockMappingDriver {
        all_class_names: names(&["Doctrine\\Tests\\Models\\Company\\Foo"]),
        ..MockMappingDriver::default()
    });

    let driver2 = Arc::new(MockMappingDriver {
        all_class_names: names(&[
            "Doctrine\\Tests\\ORM\\Mapping\\Bar",
            "Doctrine\\Tests\\ORM\\Mapping\\Baz",
            "FooBarBaz",
        ]),
        ..MockMappingDriver::default()
    });

    chain.add_driver(driver1.clone(), "Doctrine\\Tests\\Models\\Company");
    chain.add_driver(driver2.clone(), "Doctrine\\Tests\\ORM\\Mapping");

    assert_eq!(
        names(&[
            "Doctrine\\Tests\\Models\\Company\\Foo",
            "Doctrine\\Tests\\ORM\\Mapping\\Bar",
            "Doctrine\\Tests\\ORM\\Mapping\\Baz",
        ]),
        chain.get_all_class_names().unwrap()
    );
    assert_eq!(driver1.get_all_class_names_calls.load(Ordering::SeqCst), 1);
    assert_eq!(driver2.get_all_class_names_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn testIsTransient() {
    let driver1 = MockMappingDriver::stub();
    let chain = MappingDriverChain::new();

    chain.add_driver(driver1, "Doctrine\\Tests\\Models\\CMS");

    assert!(
        chain.is_transient("stdClass").unwrap(),
        "stdClass isTransient"
    );
}

#[test]
fn testDefaultDriver() {
    register();
    let company_driver = Arc::new(MockMappingDriver {
        is_transient: Some(Box::new(|_| false)),
        ..MockMappingDriver::default()
    });
    let default_driver = Arc::new(MockMappingDriver {
        is_transient: Some(Box::new(|_| true)),
        ..MockMappingDriver::default()
    });
    let entity_class_name = MODEL;
    let manager_class_name = MANAGER;
    let chain = MappingDriverChain::new();

    assert!(chain.get_default_driver().is_none());

    chain.set_default_driver(default_driver.clone());
    chain.add_driver(
        company_driver.clone(),
        "Doctrine\\Tests\\Persistence\\Mapping\\Fixtures\\Manager",
    );

    let driver = chain.get_default_driver().unwrap();
    let expected: Arc<dyn MappingDriver> = default_driver.clone();
    assert!(same(&expected, &driver));

    assert!(chain.is_transient(entity_class_name).unwrap());
    assert!(!chain.is_transient(manager_class_name).unwrap());

    assert_eq!(company_driver.load_metadata_calls(), Vec::<String>::new());
    assert_eq!(company_driver.is_transient_calls(), [manager_class_name]);
    assert_eq!(default_driver.load_metadata_calls(), Vec::<String>::new());
    assert_eq!(default_driver.is_transient_calls(), [entity_class_name]);
}

#[test]
fn testDefaultDriverGetAllClassNames() {
    let company_driver = Arc::new(MockMappingDriver {
        all_class_names: names(&["Doctrine\\Tests\\Models\\Company\\Foo"]),
        ..MockMappingDriver::default()
    });
    let default_driver = Arc::new(MockMappingDriver {
        all_class_names: names(&["Other\\Class"]),
        ..MockMappingDriver::default()
    });
    let chain = MappingDriverChain::new();

    chain.set_default_driver(default_driver.clone());
    chain.add_driver(company_driver.clone(), "Doctrine\\Tests\\Models\\Company");

    let class_names = chain.get_all_class_names().unwrap();

    assert_eq!(
        names(&["Doctrine\\Tests\\Models\\Company\\Foo", "Other\\Class"]),
        class_names
    );
    assert_eq!(
        company_driver
            .get_all_class_names_calls
            .load(Ordering::SeqCst),
        1
    );
    assert_eq!(
        default_driver
            .get_all_class_names_calls
            .load(Ordering::SeqCst),
        1
    );
}
