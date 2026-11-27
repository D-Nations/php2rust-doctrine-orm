//! Port of `Doctrine\ORM\Tools\Console\Command\SchemaTool\CreateCommand`: creates the database schema for a set of classes based on their mappings, or dumps its SQL.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/SchemaTool/CreateCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_option::{Shortcut, VALUE_NONE, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::abstract_command;
use crate::tools::console::command::abstract_entity_manager_command::{
    AbstractEntityManagerCommand, block_on, thrown,
};
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `CreateCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\SchemaTool\\CreateCommand";

const HELP: &str = r"Processes the schema and either create it directly on EntityManager Storage Connection or generate the SQL output.

<comment>Hint:</comment> If you have a database with tables that should not be managed
by the ORM, you can use a DBAL functionality to filter the tables and sequences down
on a global level:

    $config->setSchemaAssetsFilter(function (string|AbstractAsset $assetName): bool {
        if ($assetName instanceof AbstractAsset) {
            $assetName = $assetName->getName();
        }

        return !str_starts_with($assetName, 'audit_');
    });";

/// `CreateCommand`.
pub struct CreateCommand(AbstractEntityManagerCommand);

impl CreateCommand {
    /// `new CreateCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for CreateCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:schema-tool:create")?
            .set_description("Processes the schema and either create it directly on EntityManager Storage Connection or generate the SQL output")
            .add_option("em", Shortcut::None, Some(VALUE_REQUIRED), "Name of the entity manager to operate on", Value::Null, SuggestedValues::None)?
            .add_option("dump-sql", Shortcut::None, Some(VALUE_NONE), "Instead of trying to apply generated SQLs into EntityManager Storage Connection, output them.", Value::Null, SuggestedValues::None)?
            .set_help(HELP);
        Ok(())
    }

    fn execute(
        &mut self,
        _command: &mut Command,
        input: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        abstract_command::execute(
            &self.0,
            input,
            output,
            |input, output, schema_tool, metadatas| {
                let dump_sql = input.get_option("dump-sql")? == Value::Bool(true);
                let mut ui = SymfonyStyle::new(input, output)?;

                if dump_sql {
                    let sqls = schema_tool
                        .get_create_schema_sql(metadatas)
                        .map_err(thrown)?;

                    for sql in sqls {
                        ui.writeln_all(&[format!("{sql};")], 1)?;
                    }

                    return Ok(0);
                }

                let mut notification_ui = ui.get_error_style()?;

                notification_ui.caution(&[
                    "This operation should not be executed in a production environment!",
                ])?;

                notification_ui.text(&["Creating database schema..."])?;
                notification_ui.new_line(1)?;

                block_on(schema_tool.create_schema(metadatas))?.map_err(thrown)?;

                notification_ui.success(&["Database schema created successfully!"])?;

                Ok(0)
            },
        )
    }
}
