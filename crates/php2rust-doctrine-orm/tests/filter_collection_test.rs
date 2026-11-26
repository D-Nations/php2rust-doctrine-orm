//! Upstream: doctrine/orm tests/Tests/ORM/Query/FilterCollectionTest.php
//!
//! `getTestEntityManager()` is an entity manager on an `SQLite`
//! connection; `MyFilter` registers its constructor
//! (`register_filter_class()`), as the configuration names it by class.
//! `assertContainsOnly(MyFilter::class, ...)` compares the filters'
//! classes.

#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;

use std::sync::Arc;

use php_value::Thrown;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::ClassMetadataInfo;
use php2rust_doctrine_orm::query::{FilterConstraint, SqlFilter, register_filter_class};

const MY_FILTER: &str = "Doctrine\\Tests\\ORM\\Query\\MyFilter";

/// `MyFilter extends SQLFilter`.
struct MyFilter;

impl FilterConstraint for MyFilter {
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        _target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        // getParameter applies quoting automatically
        Ok(format!(
            "{target_table_alias}.id = {}",
            filter.get_parameter("id")?
        ))
    }
}

fn set_up() -> Arc<EntityManager> {
    register_filter_class(
        MY_FILTER,
        Arc::new(|em| SqlFilter::new(em, MY_FILTER, Box::new(MyFilter))),
    );
    let em = orm::entity_manager(orm::connection(), None);
    em.get_configuration().add_filter("testFilter", MY_FILTER);
    em
}

fn contains_only_my_filter(filters: &[(String, Arc<SqlFilter>)]) -> bool {
    filters
        .iter()
        .all(|(_, filter)| filter.class() == MY_FILTER)
}

#[test]
fn testEnable() {
    let em = set_up();
    let filter_collection = em.get_filters();

    assert_eq!(filter_collection.get_enabled_filters().len(), 0);

    let filter1 = filter_collection.enable("testFilter").unwrap();

    let enabled_filters = filter_collection.get_enabled_filters();

    assert_eq!(enabled_filters.len(), 1);
    assert!(contains_only_my_filter(&enabled_filters));

    let filter2 = filter_collection.disable("testFilter").unwrap();
    assert_eq!(filter_collection.get_enabled_filters().len(), 0);
    assert!(Arc::ptr_eq(&filter1, &filter2));

    let filter3 = filter_collection.enable("testFilter").unwrap();
    assert!(!Arc::ptr_eq(&filter1, &filter3));

    let filter4 = filter_collection.suspend("testFilter").unwrap();
    assert!(Arc::ptr_eq(&filter3, &filter4));

    let filter5 = filter_collection.enable("testFilter").unwrap();
    assert!(!Arc::ptr_eq(&filter4, &filter5));

    assert_eq!(enabled_filters.len(), 1);
    assert!(contains_only_my_filter(&enabled_filters));
}

#[test]
fn testSuspend() {
    let em = set_up();
    let filter_collection = em.get_filters();

    assert_eq!(filter_collection.get_enabled_filters().len(), 0);

    let filter1 = filter_collection.enable("testFilter").unwrap();
    assert_eq!(filter_collection.get_enabled_filters().len(), 1);

    let filter2 = filter_collection.suspend("testFilter").unwrap();
    assert!(Arc::ptr_eq(&filter1, &filter2));
    assert_eq!(filter_collection.get_enabled_filters().len(), 0);

    let filter3 = filter_collection.restore("testFilter").unwrap();
    assert!(Arc::ptr_eq(&filter1, &filter3));
    assert_eq!(filter_collection.get_enabled_filters().len(), 1);
}

#[test]
fn testRestoreFailure() {
    let em = set_up();
    let filter_collection = em.get_filters();

    let e = filter_collection.suspend("testFilter").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");
}

#[test]
fn testHasFilter() {
    let em = set_up();
    let filter_collection = em.get_filters();

    assert!(filter_collection.has("testFilter").unwrap());
    assert!(!filter_collection.has("fakeFilter").unwrap());
}

#[test]
fn testIsEnabled() {
    let em = set_up();
    let filter_collection = em.get_filters();

    assert!(!filter_collection.is_enabled("testFilter"));

    filter_collection.enable("testFilter").unwrap();

    assert!(filter_collection.is_enabled("testFilter"));

    filter_collection.suspend("testFilter").unwrap();

    assert!(!filter_collection.is_enabled("testFilter"));

    filter_collection.restore("testFilter").unwrap();

    assert!(filter_collection.is_enabled("testFilter"));

    assert!(!filter_collection.is_enabled("wrongFilter"));
}

#[test]
fn testIsSuspended() {
    let em = set_up();
    let filter_collection = em.get_filters();

    assert!(!filter_collection.is_suspended("testFilter"));

    filter_collection.enable("testFilter").unwrap();

    assert!(!filter_collection.is_suspended("testFilter"));

    filter_collection.suspend("testFilter").unwrap();

    assert!(filter_collection.is_suspended("testFilter"));

    filter_collection.restore("testFilter").unwrap();

    assert!(!filter_collection.is_suspended("testFilter"));

    filter_collection.disable("testFilter").unwrap();

    assert!(!filter_collection.is_suspended("testFilter"));

    assert!(!filter_collection.is_suspended("wrongFilter"));
}

#[test]
fn testGetFilterInvalidArgument() {
    let em = set_up();
    let filter_collection = em.get_filters();
    let e = filter_collection.get_filter("testFilter").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");
}

#[test]
fn testGetFilter() {
    let em = set_up();
    let filter_collection = em.get_filters();
    filter_collection.enable("testFilter").unwrap();

    assert_eq!(
        filter_collection.get_filter("testFilter").unwrap().class(),
        MY_FILTER
    );

    filter_collection.suspend("testFilter").unwrap();

    let e = filter_collection.get_filter("testFilter").unwrap_err();
    assert_eq!(e.class, "InvalidArgumentException");
}

#[test]
fn testHashing() {
    let em = set_up();
    let filter_collection = em.get_filters();

    assert!(filter_collection.is_clean());

    let old_hash = filter_collection.get_hash();
    filter_collection.set_filters_state_dirty();

    assert!(!filter_collection.is_clean());
    assert_eq!(old_hash, filter_collection.get_hash());
    assert!(filter_collection.is_clean());

    filter_collection.enable("testFilter").unwrap();

    assert!(!filter_collection.is_clean());

    let hash = filter_collection.get_hash();

    assert_ne!(old_hash, hash);
    assert!(filter_collection.is_clean());
    assert_eq!(hash, filter_collection.get_hash());

    filter_collection.suspend("testFilter").unwrap();

    assert!(!filter_collection.is_clean());
    assert_eq!(old_hash, filter_collection.get_hash());
    assert!(filter_collection.is_clean());

    filter_collection.restore("testFilter").unwrap();

    assert!(!filter_collection.is_clean());
    assert_eq!(hash, filter_collection.get_hash());
    assert!(filter_collection.is_clean());

    filter_collection.disable("testFilter").unwrap();

    assert!(!filter_collection.is_clean());
    assert_eq!(old_hash, filter_collection.get_hash());
    assert!(filter_collection.is_clean());
}
