//! Port of `ClearCacheEntityRegionCommandTest`. `OrmFunctionalTestCase` with the second-level cache is
//! `SecondLevelCacheFunctionalTestCase`'s fixture without its data.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Console/Command/ClearCacheEntityRegionCommandTest.php
#![cfg(feature = "console")]
#![allow(non_snake_case)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

#[path = "common/console.rs"]
mod console;
#[path = "common/models.rs"]
mod models;
#[path = "common/orm.rs"]
mod orm;
#[path = "common/query_log.rs"]
mod query_log;
#[path = "common/slc.rs"]
mod slc;
#[path = "common/slc_functional.rs"]
mod slc_functional;

use php_value::Value;
use php2rust_doctrine_orm::tools::console::command::clear_cache::EntityRegionCommand;
use symfony_console::application::Application;

const NAME: &str = "orm:clear-cache:region:entity";

/// `setUp()`.
async fn set_up() -> Application {
    let fixture = slc_functional::set_up().await;
    let command = EntityRegionCommand::command(console::provider(&fixture.em)).unwrap();
    let mut application = Application::default();
    console::add_command_to_application(&mut application, command);
    application
}

#[tokio::test(flavor = "multi_thread")]
async fn testClearAllRegion() {
    let mut application = set_up().await;
    let display = console::run(&mut application, NAME, &[("--all", Value::from(true))]);

    assert!(display.contains(" // Clearing all second-level cache entity regions"));
}

#[tokio::test(flavor = "multi_thread")]
async fn testClearByEntityClassName() {
    let mut application = set_up().await;
    let display = console::run(
        &mut application,
        NAME,
        &[("entity-class", Value::from(slc::COUNTRY))],
    );

    assert!(display.contains(
        " // Clearing second-level cache for entity \"Doctrine\\Tests\\Models\\Cache\\Country\""
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn testClearCacheEntryName() {
    let mut application = set_up().await;
    let display = console::run(
        &mut application,
        NAME,
        &[
            ("entity-class", Value::from(slc::COUNTRY)),
            ("entity-id", Value::Int(1)),
        ],
    );

    assert!(display.contains(
        "Clearing second-level cache entry for entity \"Doctrine\\Tests\\Models\\Cache\\Country\" identified by \"1\""
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn testFlushRegionName() {
    let mut application = set_up().await;
    let display = console::run(
        &mut application,
        NAME,
        &[
            ("entity-class", Value::from(slc::COUNTRY)),
            ("--flush", Value::from(true)),
        ],
    );

    assert!(display.contains(
        " // Flushing cache provider configured for entity named \"Doctrine\\Tests\\Models\\Cache\\Country\""
    ));
}
