//! Port of `MappingDescribeCommandTest`. `OrmFunctionalTestCase`'s
//! attribute driver reads `tests/Tests/Models`: here a locator of the
//! recorded classes in `Doctrine\Tests\Models\`. `testComplete()` runs
//! both of its data sets.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Console/Command/MappingDescribeCommandTest.php
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

use std::sync::Arc;

use php_value::{Thrown, Value};
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_orm::tools::console::command::MappingDescribeCommand;
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;
use php2rust_doctrine_persistence::php_reflection::{self, ClassKind};
use symfony_console::application::{Application, SharedCommand};
use symfony_console::tester::command_completion_tester::CommandCompletionTester;
use symfony_console::tester::command_tester::{CommandTester, ExecuteOptions};

const ATTRACTION_INFO: &str = "Doctrine\\Tests\\Models\\Cache\\AttractionInfo";

/// The classes of `tests/Tests/Models`.
#[derive(Debug)]
struct ModelsLocator;

impl ClassLocator for ModelsLocator {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(php_reflection::declared()
            .iter()
            .filter(|class| matches!(class.kind(), ClassKind::Class | ClassKind::Enum))
            .map(|class| class.name().to_owned())
            .filter(|name| name.starts_with("Doctrine\\Tests\\Models\\"))
            .collect())
    }
}

fn entity_manager() -> Arc<EntityManager> {
    orm::entity_manager_with(orm::connection(), |c| {
        let locator: Arc<dyn ClassLocator> = Arc::new(ModelsLocator);
        c.set_metadata_driver_impl(Arc::new(
            AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
        ));
    })
}

struct Test {
    command: SharedCommand,
    tester: CommandTester,
    em: Arc<EntityManager>,
}

/// `setUp()`.
fn set_up() -> Test {
    let em = entity_manager();
    let mut application = Application::default();
    console::add_command_to_application(
        &mut application,
        MappingDescribeCommand::command(console::provider(&em)).unwrap(),
    );
    let command = application.find("orm:mapping:describe").unwrap();
    let tester = CommandTester::new(command.clone());
    Test {
        command,
        tester,
        em,
    }
}

impl Test {
    fn execute(&mut self, items: &[(&str, Value)]) -> Result<String, String> {
        symfony_console::env::putenv("COLUMNS", Some("120"));
        let input = console::command_input(&self.command, items);
        self.tester
            .execute(input, ExecuteOptions::default())
            .map_err(|e| format!("{}: {}", e.class(), e.message()))?;
        Ok(self.tester.get_display(false).unwrap())
    }
}

#[test]
fn testShowSpecificFuzzySingle() {
    let mut test = set_up();
    let display = test
        .execute(&[("entityName", Value::from("AttractionInfo"))])
        .unwrap();

    assert!(display.contains(ATTRACTION_INFO));
    assert!(display.contains("Root entity name"));
}

#[test]
fn testShowSpecificFuzzySingleJson() {
    let mut test = set_up();
    let display = test
        .execute(&[
            ("entityName", Value::from("AttractionInfo")),
            ("--format", Value::from("json")),
        ])
        .unwrap();
    let decoded_json: serde_json::Value = serde_json::from_str(&display).unwrap();

    assert_eq!(ATTRACTION_INFO, decoded_json["name"]);
    assert!(decoded_json.get("rootEntityName").is_some());
    assert!(decoded_json.get("fieldMappings").is_some());
    assert!(decoded_json.get("associationMappings").is_some());
    assert!(decoded_json["fieldMappings"].get("id").is_some());
}

#[test]
fn testShowSpecificFuzzyAmbiguous() {
    let mut test = set_up();
    let error = test
        .execute(&[("entityName", Value::from("Attraction"))])
        .unwrap_err();

    assert!(error.starts_with("InvalidArgumentException: "));
    assert!(error.contains("possible matches"));
}

#[test]
fn testShowSpecificNotFound() {
    let mut test = set_up();
    let error = test
        .execute(&[("entityName", Value::from("AttractionFooBar"))])
        .unwrap_err();

    assert!(error.starts_with("InvalidArgumentException: "));
    assert!(
        error.contains("Could not find any mapped Entity classes matching \"AttractionFooBar\"")
    );
}

/// `provideCompletionSuggestions()`.
fn provide_completion_suggestions() -> Vec<(&'static str, Vec<&'static str>, Vec<&'static str>)> {
    vec![
        (
            "entityName",
            vec![""],
            vec![
                "Doctrine\\\\Tests\\\\Models\\\\Cache\\\\Restaurant",
                "Doctrine\\\\Tests\\\\Models\\\\Cache\\\\Beach",
                "Doctrine\\\\Tests\\\\Models\\\\Cache\\\\Bar",
            ],
        ),
        (
            "format option value",
            vec!["--format="],
            vec!["text", "json"],
        ),
    ]
}

#[test]
fn testComplete() {
    for (name, input, expected_suggestions) in provide_completion_suggestions() {
        let test = set_up();
        let completion_tester = CommandCompletionTester::new(
            MappingDescribeCommand::command(console::provider(&test.em)).unwrap(),
        );

        let suggestions = completion_tester.complete(&input).unwrap();

        for expected in expected_suggestions {
            assert!(
                suggestions.iter().any(|s| s == expected),
                "{name}: {expected}"
            );
        }
    }
}

#[test]
fn testShowEntityWithEntityListeners() {
    let mut test = set_up();
    let display = test
        .execute(&[("entityName", Value::from(orm::CMS_ADDRESS))])
        .unwrap();

    assert!(display.contains(orm::CMS_ADDRESS));
    assert!(display.contains("Entity listeners"));
    assert!(display.contains("CmsAddressListener"));
    assert!(display.contains("postPersist"));
    assert!(display.contains("prePersist"));
}
