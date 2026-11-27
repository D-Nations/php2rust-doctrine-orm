//! Port of `Doctrine\ORM\Tools\Console\Command\SchemaTool\DropCommand`: drops the database schema for a set of classes based on their mappings, or dumps its SQL.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/SchemaTool/DropCommand.php

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

/// `DropCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\SchemaTool\\DropCommand";

const HELP: &str = r"Processes the schema and either drop the database schema of EntityManager Storage Connection or generate the SQL output.
Beware that the complete database is dropped by this command, even tables that are not relevant to your metadata model.

<comment>Hint:</comment> If you have a database with tables that should not be managed
by the ORM, you can use a DBAL functionality to filter the tables and sequences down
on a global level:

    $config->setSchemaAssetsFilter(function (string|AbstractAsset $assetName): bool {
        if ($assetName instanceof AbstractAsset) {
            $assetName = $assetName->getName();
        }

        return !str_starts_with($assetName, 'audit_');
    });";

/// `DropCommand`.
pub struct DropCommand(AbstractEntityManagerCommand);

impl DropCommand {
    /// `new DropCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for DropCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:schema-tool:drop")?
            .set_description("Drop the complete database schema of EntityManager Storage Connection or generate the corresponding SQL output")
            .add_option("em", Shortcut::None, Some(VALUE_REQUIRED), "Name of the entity manager to operate on", Value::Null, SuggestedValues::None)?
            .add_option("dump-sql", Shortcut::None, Some(VALUE_NONE), "Instead of trying to apply generated SQLs into EntityManager Storage Connection, output them.", Value::Null, SuggestedValues::None)?
            .add_option("force", Shortcut::One("f"), Some(VALUE_NONE), "Don't ask for the deletion of the database, but force the operation to run.", Value::Null, SuggestedValues::None)?
            .add_option("full-database", Shortcut::None, Some(VALUE_NONE), "Instead of using the Class Metadata to detect the database table schema, drop ALL assets that the database contains.", Value::Null, SuggestedValues::None)?
            .set_help(HELP);
        Ok(())
    }

    fn execute(
        &mut self,
        command: &mut Command,
        input: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        let name = command.get_name().unwrap_or_default().to_owned();
        abstract_command::execute(
            &self.0,
            input,
            output,
            |input, output, schema_tool, metadatas| {
                let is_full_database_drop = input.get_option("full-database")?.is_truthy();
                let dump_sql = input.get_option("dump-sql")? == Value::Bool(true);
                let force = input.get_option("force")? == Value::Bool(true);
                let drop_sql = || -> Result<Vec<String>> {
                    if is_full_database_drop {
                        block_on(schema_tool.get_drop_database_sql())?.map_err(thrown)
                    } else {
                        schema_tool.get_drop_schema_sql(metadatas).map_err(thrown)
                    }
                };
                let mut ui = SymfonyStyle::new(input, output)?;

                if dump_sql {
                    for sql in drop_sql()? {
                        ui.writeln_all(&[format!("{sql};")], 1)?;
                    }

                    return Ok(0);
                }

                let mut notification_ui = ui.get_error_style()?;

                if force {
                    notification_ui.text(&["Dropping database schema..."])?;
                    notification_ui.new_line(1)?;

                    if is_full_database_drop {
                        block_on(schema_tool.drop_database())?.map_err(thrown)?;
                    } else {
                        block_on(schema_tool.drop_schema(metadatas))?.map_err(thrown)?;
                    }

                    notification_ui.success(&["Database schema dropped successfully!"])?;

                    return Ok(0);
                }

                notification_ui.caution(&[
                    "This operation should not be executed in a production environment!",
                ])?;

                let sqls = drop_sql()?;

                if sqls.is_empty() {
                    notification_ui.success(&["Nothing to drop. The database is empty!"])?;

                    return Ok(0);
                }

                notification_ui.text(&[
                &format!(
                    "The Schema-Tool would execute <info>\"{}\"</info> queries to update the database.",
                    sqls.len()
                ),
                "",
                "Please run the operation by passing one - or both - of the following options:",
                "",
                &format!("    <info>{name} --force</info> to execute the command"),
                &format!("    <info>{name} --dump-sql</info> to dump the SQL statements to the screen"),
            ])?;

                Ok(1)
            },
        )
    }
}
