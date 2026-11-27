//! The console test helpers: `ApplicationCompatibility::addCommandToApplication()`
//! and a `CommandTester` run with its display. The tests run with
//! upstream's phpunit.xml.dist environment (`COLUMNS=120`).
//!
//! Upstream: doctrine/orm src/Tools/Console/ApplicationCompatibility.php

#![allow(dead_code, reason = "each test binary uses a part of the helpers")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::sync::Arc;

use php_value::{PhpArray, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::tools::console::{EntityManagerProvider, SingleManagerProvider};
use symfony_console::application::{Application, SharedCommand};
use symfony_console::command::command::Command;
use symfony_console::tester::command_tester::{CommandTester, ExecuteOptions};

/// `new SingleManagerProvider($em)`.
pub fn provider(em: &Arc<EntityManager>) -> Arc<dyn EntityManagerProvider> {
    Arc::new(SingleManagerProvider::new(Arc::clone(em)))
}

/// `self::addCommandToApplication($application, $command)`.
pub fn add_command_to_application(
    application: &mut Application,
    command: Command,
) -> SharedCommand {
    application.add_command(command).unwrap().unwrap()
}

/// The input array of `$tester->execute()`.
pub fn input(items: &[(&str, Value)]) -> PhpArray {
    let mut input = PhpArray::new();
    for (key, value) in items {
        input.insert(*key, value.clone());
    }
    input
}

/// `['command' => $command->getName(), ...]`.
pub fn command_input(command: &SharedCommand, items: &[(&str, Value)]) -> PhpArray {
    let name = command.lock().get_name().unwrap().to_owned();
    let mut all = vec![("command", Value::from(name))];
    all.extend(items.iter().cloned());
    input(&all)
}

/// `['decorated' => false]`.
pub fn undecorated() -> ExecuteOptions {
    ExecuteOptions {
        decorated: Some(false),
        ..ExecuteOptions::default()
    }
}

/// `$tester->execute($input, $options)` and `$tester->getDisplay()`, with
/// phpunit.xml.dist's `COLUMNS=120`.
pub fn execute(
    tester: &mut CommandTester,
    input: PhpArray,
    options: ExecuteOptions,
) -> (i64, String) {
    symfony_console::env::putenv("COLUMNS", Some("120"));
    let code = tester.execute(input, options).unwrap();
    (code, tester.get_display(false).unwrap())
}

/// Finds `name`, runs it with `items` after `command`, and returns the display.
pub fn run(application: &mut Application, name: &str, items: &[(&str, Value)]) -> String {
    let command = application.find(name).unwrap();
    let mut tester = CommandTester::new(command.clone());
    execute(&mut tester, command_input(&command, items), undecorated()).1
}

/// Finds `name` and runs it with `items` after `command` and no
/// execute options, returning the display.
pub fn run_default(application: &mut Application, name: &str, items: &[(&str, Value)]) -> String {
    let command = application.find(name).unwrap();
    let mut tester = CommandTester::new(command.clone());
    execute(
        &mut tester,
        command_input(&command, items),
        ExecuteOptions::default(),
    )
    .1
}
