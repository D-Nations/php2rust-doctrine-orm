//! Port of `DropCommandTest` and its `CommandTestCase`: the entity
//! manager's attribute driver reads the `Models` directory next to the
//! test (the recorded `Keyboard`), on the shared connection.
//!
//! Upstream: doctrine/orm tests/Tests/ORM/Tools/Console/Command/SchemaTool/DropCommandTest.php tests/Tests/ORM/Tools/Console/Command/SchemaTool/CommandTestCase.php
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

use php_value::{PhpArray, Thrown, Value};
use php2rust_doctrine_dbal::ParameterTypes;
use php2rust_doctrine_orm::entity_manager::EntityManager;
use php2rust_doctrine_orm::mapping::driver::AttributeDriver;
use php2rust_doctrine_orm::mapping::driver::attribute_driver::Paths;
use php2rust_doctrine_orm::tools::console::command::schema_tool::DropCommand;
use php2rust_doctrine_persistence::mapping::driver::ClassLocator;
use symfony_console::tester::command_tester::{CommandTester, ExecuteOptions};

const KEYBOARD: &str =
    "Doctrine\\Tests\\ORM\\Tools\\Console\\Command\\SchemaTool\\Models\\Keyboard";

/// `AttributeDriverFactory::createAttributeDriver([__DIR__ . '/Models'])`.
#[derive(Debug)]
struct ModelsLocator;

impl ClassLocator for ModelsLocator {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(vec![KEYBOARD.to_owned()])
    }
}

/// `getEntityManager(null, $attributeDriver)`.
fn entity_manager() -> Arc<EntityManager> {
    orm::entity_manager_with(orm::connection(), |c| {
        let locator: Arc<dyn ClassLocator> = Arc::new(ModelsLocator);
        c.set_metadata_driver_impl(Arc::new(
            AttributeDriver::new(Paths::Locator(locator), true).unwrap(),
        ));
    })
}

/// `getCommandTester($commandClass)`.
fn get_command_tester(entity_manager: &Arc<EntityManager>) -> CommandTester {
    CommandTester::new(DropCommand::command(console::provider(entity_manager)).unwrap())
}

/// `self::$sharedConn->executeStatement($sql)`.
async fn execute_statement(entity_manager: &EntityManager, sql: &str) {
    entity_manager
        .get_connection()
        .execute_statement(sql, &PhpArray::new(), &ParameterTypes::new())
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn testItPrintsTheSql() {
    let entity_manager = entity_manager();
    orm::create_schema(&entity_manager, &[KEYBOARD]).await;
    let mut tester = get_command_tester(&entity_manager);
    let (_, display) = console::execute(
        &mut tester,
        console::input(&[("--dump-sql", Value::from(true))]),
        ExecuteOptions::default(),
    );

    execute_statement(&entity_manager, &display).await;
}
