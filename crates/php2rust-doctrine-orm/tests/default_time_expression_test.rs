//! Port of tests/Tests/ORM/Functional/DefaultTimeExpressionTest.php and
//! DefaultTimeExpressionXmlTest.php, on `SQLite` with the recorded models
//! (the XML test with the XML driver on the test mapping files).
//! `VerifyDeprecations` counts the ORM's and DBAL's deprecations (each
//! port tracks its own). `DefaultExpression::toSQL()` exists, so
//! `testNoDeprecationsAreTrownWhenTheyCannotBeAddressed` is skipped
//! upstream and is not ported.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/DefaultTimeExpressionTest.php
//! Upstream: doctrine/orm tests/Tests/ORM/Functional/DefaultTimeExpressionXmlTest.php

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

use php2rust_doctrine_orm::deprecations;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::xml_driver::{DEFAULT_FILE_EXTENSION, XmlDriver};

use orm::{get, new_object};

const NS: &str = "Doctrine\\Tests\\ORM\\Functional\\";
const ORM_DEPRECATION: &str = "https://github.com/doctrine/orm/issues/12252";
const DBAL_DEPRECATION: &str = "https://github.com/doctrine/dbal/pull/7195";

/// `expectDeprecationWithIdentifier()` and
/// `expectNoDeprecationWithIdentifier()`, checked at the end.
struct Deprecations {
    orm: u64,
    dbal: u64,
}

impl Deprecations {
    fn start() -> Self {
        deprecations::enable_tracking_deprecations();
        php2rust_doctrine_dbal::deprecations::enable_tracking_deprecations();
        Self {
            orm: deprecations::count(ORM_DEPRECATION),
            dbal: php2rust_doctrine_dbal::deprecations::count(DBAL_DEPRECATION),
        }
    }

    fn orm_triggered(&self) -> bool {
        deprecations::count(ORM_DEPRECATION) > self.orm
    }

    fn dbal_triggered(&self) -> bool {
        php2rust_doctrine_dbal::deprecations::count(DBAL_DEPRECATION) > self.dbal
    }
}

fn class(short: &str) -> String {
    format!("{NS}{short}")
}

/// The platform's checks: `CURRENT_TIMESTAMP`, `CURRENT_TIME` and
/// `CURRENT_DATE` (`SQLite` is not a `MySQL` platform).
fn assert_standard_platform(em: &EntityManager) {
    let platform = em.get_platform();
    assert_eq!(platform.get_current_timestamp_sql(), "CURRENT_TIMESTAMP");
    assert_eq!(platform.get_current_time_sql(), "CURRENT_TIME");
    assert_eq!(platform.get_current_date_sql(), "CURRENT_DATE");
}

/// `createSchemaForModels($class)`, `persist(new $class())`, `flush()`
/// (with `clear()` when asked) and `find($class, $entity->id)`.
async fn persist_and_find(em: &Arc<EntityManager>, class_name: &str, clear: bool) {
    orm::create_schema(em, &[class_name]).await;
    let entity = new_object(class_name);
    em.persist(&entity).unwrap();
    em.flush().await.unwrap();
    if clear {
        em.clear().unwrap();
    }
    em.find(class_name, get(&entity, "id")).await.unwrap();
}

/// `getEntityManager(mappingDriver: new XmlDriver(__DIR__ . '/../Mapping/xml/'))`.
fn xml_entity_manager() -> Arc<EntityManager> {
    models::register();
    let directory = format!("{}/tests/data/Mapping/xml", env!("CARGO_MANIFEST_DIR"));
    orm::entity_manager_with(orm::connection(), |configuration| {
        configuration.set_metadata_driver_impl(Arc::new(XmlDriver::create(
            directory.as_str(),
            DEFAULT_FILE_EXTENSION,
            true,
        )));
    })
}

#[tokio::test]
async fn testUsingTimeRelatedDefaultExpressionCausesAnOrmDeprecationAndNoDbalDeprecation() {
    let em = orm::entity_manager(orm::connection(), None);
    assert_standard_platform(&em);

    let deprecations = Deprecations::start();

    persist_and_find(&em, &class("LegacyTimeEntity"), false).await;

    assert!(deprecations.orm_triggered());
    assert!(!deprecations.dbal_triggered());
}

#[tokio::test]
async fn testUsingDefaultExpressionInstancesCausesNoDeprecation() {
    let em = orm::entity_manager(orm::connection(), None);

    let deprecations = Deprecations::start();

    persist_and_find(&em, &class("TimeEntity"), false).await;

    assert!(!deprecations.orm_triggered());
    assert!(!deprecations.dbal_triggered());
}

/// `DefaultTimeExpressionXmlTest`.
#[tokio::test]
async fn testUsingTimeRelatedDefaultExpressionCausesAnOrmDeprecationAndNoDbalDeprecationXml() {
    let em = xml_entity_manager();
    assert_standard_platform(&em);

    let deprecations = Deprecations::start();

    persist_and_find(&em, &class("XmlLegacyTimeEntity"), false).await;

    assert!(deprecations.orm_triggered());
    assert!(!deprecations.dbal_triggered());
}

/// `DefaultTimeExpressionXmlTest`.
#[tokio::test]
async fn testUsingDefaultExpressionInstancesCausesNoDeprecationXmlDriver() {
    let em = xml_entity_manager();

    let deprecations = Deprecations::start();

    persist_and_find(&em, &class("XmlTimeEntity"), true).await;

    assert!(!deprecations.orm_triggered());
    assert!(!deprecations.dbal_triggered());
}
