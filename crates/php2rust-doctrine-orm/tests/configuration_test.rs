//! Upstream: doctrine/orm tests/Tests/ORM/ConfigurationTest.php
//!
//! Only the filter test: the rest of `ConfigurationTest` is pending.

#![allow(non_snake_case)]

use php2rust_doctrine_orm::configuration::Configuration;

const CONFIGURATION_TEST: &str = "Doctrine\\Tests\\ORM\\ConfigurationTest";

#[test]
fn testAddGetFilters() {
    let configuration = Configuration::new();
    assert_eq!(
        configuration.get_filter_class_name("NonExistingFilter"),
        None
    );
    configuration.add_filter("FilterName", CONFIGURATION_TEST);
    assert_eq!(
        configuration.get_filter_class_name("FilterName").as_deref(),
        Some(CONFIGURATION_TEST)
    );
}
