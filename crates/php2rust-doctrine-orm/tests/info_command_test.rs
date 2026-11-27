//! Port of `InfoCommandTest`. The entity manager stubs are entity
//! managers on a stub mapping driver: its `getAllClassNames()` returns
//! the stubbed names, and loading the metadata of `InvalidEntity` throws
//! the stubbed `MappingException`, which `getClassMetadata()` then
//! throws (the class `InvalidEntity` is declared, so that the factory
//! reaches the driver).
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Console/Command/InfoCommandTest.php
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

use std::sync::Arc;

use php_value::Thrown;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::mapping_exception;
use php2rust_doctrine_orm::tools::console::command::InfoCommand;
use php2rust_doctrine_persistence::mapping::ClassMetadata;
use php2rust_doctrine_persistence::mapping::driver::MappingDriver;
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};
use symfony_console::application::Application;

/// `$this->createStub(MappingDriver::class)` with `getAllClassNames()`.
struct MappingDriverStub(Vec<String>);

impl MappingDriver for MappingDriverStub {
    fn load_metadata_for_class(
        &self,
        _class_name: &str,
        _metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        Err(Thrown::new(
            mapping_exception::CLASS,
            "exception message",
            0,
        ))
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(self.0.clone())
    }

    fn is_transient(&self, _class_name: &str) -> Result<bool, Thrown> {
        Ok(false)
    }
}

/// The entity manager stub whose configuration has the stub driver.
fn stub_entity_manager(class_names: &[&str]) -> Arc<EntityManager> {
    let driver = MappingDriverStub(class_names.iter().map(|&n| n.to_owned()).collect());
    orm::entity_manager_with(orm::connection(), move |c| {
        c.set_metadata_driver_impl(Arc::new(driver));
    })
}

fn application(em: &Arc<EntityManager>) -> Application {
    let mut application = Application::default();
    console::add_command_to_application(
        &mut application,
        InfoCommand::command(console::provider(em)).unwrap(),
    );
    application
}

#[tokio::test(flavor = "multi_thread")]
async fn testListAllClasses() {
    let fixture = slc_functional::set_up().await;
    let mut application = application(&fixture.em);

    let display = console::run_default(&mut application, "orm:info", &[]);

    assert!(display.contains(slc_functional::ATTRACTION_INFO));
    assert!(display.contains(slc::CITY));
}

#[test]
fn testEmptyEntityClassNames() {
    let em = stub_entity_manager(&[]);
    let mut application = application(&em);

    let display = console::run_default(&mut application, "orm:info", &[]);

    assert!(display.contains(
        " ! [CAUTION] You do not have any mapped Doctrine ORM entities according to the current configuration"
    ));

    assert!(display.contains(
        " !           If you have entities or mapping files you should check your mapping configuration for errors."
    ));
}

#[test]
fn testInvalidEntityClassMetadata() {
    // The class exists: the stub's getClassMetadata() is the driver's.
    register_class(ClassInfo::class("InvalidEntity"));
    let em = stub_entity_manager(&["InvalidEntity"]);
    let mut application = application(&em);

    let display = console::run_default(&mut application, "orm:info", &[]);

    assert!(display.contains("[FAIL] InvalidEntity"), "{display}");
    assert!(display.contains("exception message"));
}
