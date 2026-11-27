//! Port of `Doctrine\ORM\Tools\Console\Command\SchemaTool\UpdateCommand`: executes (or dumps) the SQL that brings the database schema in line with the mapping metadata. The `--dump-sql` lines and the notifications are written by two styles on the output, as upstream's `$ui` and `$notificationUi`; the notification style of the `--complete` warning is not the one of the `--force` messages.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/SchemaTool/UpdateCommand.php

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

/// `UpdateCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\SchemaTool\\UpdateCommand";

const HELP: &str = r#"The <info>%command.name%</info> command generates the SQL needed to
synchronize the database schema with the current mapping metadata of the
default entity manager.

For example, if you add metadata for a new column to an entity, this command
would generate and output the SQL needed to add the new column to the database:

<info>%command.name% --dump-sql</info>

Alternatively, you can execute the generated queries:

<info>%command.name% --force</info>

If both options are specified, the queries are output and then executed:

<info>%command.name% --dump-sql --force</info>

Finally, be aware that this task will drop all database assets (e.g. tables,
etc) that are *not* described by the current metadata. In other words, without
this option, this task leaves untouched any "extra" tables that exist in the
database, but which aren't described by any metadata.

<comment>Hint:</comment> If you have a database with tables that should not be managed
by the ORM, you can use a DBAL functionality to filter the tables and sequences down
on a global level:

    $config->setSchemaAssetsFilter(function (string|AbstractAsset $assetName): bool {
        if ($assetName instanceof AbstractAsset) {
            $assetName = $assetName->getName();
        }

        return !str_starts_with($assetName, 'audit_');
    });"#;

/// `UpdateCommand`.
pub struct UpdateCommand(AbstractEntityManagerCommand);

impl UpdateCommand {
    /// `new UpdateCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

/// The deprecation of `--complete`.
pub const COMPLETE_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/11354";

impl CommandHandler for UpdateCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:schema-tool:update")?
            .set_description("Executes (or dumps) the SQL needed to update the database schema to match the current mapping metadata")
            .add_option("em", Shortcut::None, Some(VALUE_REQUIRED), "Name of the entity manager to operate on", Value::Null, SuggestedValues::None)?
            .add_option("complete", Shortcut::None, Some(VALUE_NONE), "This option is a no-op, is deprecated and will be removed in 4.0", Value::Null, SuggestedValues::None)?
            .add_option("dump-sql", Shortcut::None, Some(VALUE_NONE), "Dumps the generated SQL statements to the screen (does not execute them).", Value::Null, SuggestedValues::None)?
            .add_option("force", Shortcut::One("f"), Some(VALUE_NONE), "Causes the generated SQL statements to be physically executed against your database.", Value::Null, SuggestedValues::None)?
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
                {
                    let mut notification_ui =
                        SymfonyStyle::new(input, &mut *output)?.get_error_style()?;

                    if input.get_option("complete")? == Value::Bool(true) {
                        crate::deprecations::trigger(COMPLETE_DEPRECATION, || {
                            "The --complete option is a no-op, is deprecated and will be removed in Doctrine ORM 4.0.".to_owned()
                        });
                        notification_ui.warning(&["The --complete option is a no-op, is deprecated and will be removed in Doctrine ORM 4.0."])?;
                    }
                }

                let sqls =
                    block_on(schema_tool.get_update_schema_sql(metadatas))?.map_err(thrown)?;

                if sqls.is_empty() {
                    SymfonyStyle::new(input, output)?
                    .get_error_style()?
                    .success(&["Nothing to update - your database is already in sync with the current entity metadata."])?;

                    return Ok(0);
                }

                let dump_sql = input.get_option("dump-sql")? == Value::Bool(true);
                let force = input.get_option("force")? == Value::Bool(true);

                if dump_sql {
                    let mut ui = SymfonyStyle::new(input, &mut *output)?;
                    for sql in &sqls {
                        ui.writeln_all(&[format!("{sql};")], 1)?;
                    }
                }

                let mut notification_ui = SymfonyStyle::new(input, output)?.get_error_style()?;

                if force {
                    if dump_sql {
                        notification_ui.new_line(1)?;
                    }

                    notification_ui.text(&["Updating database schema..."])?;
                    notification_ui.new_line(1)?;

                    block_on(schema_tool.update_schema(metadatas))?.map_err(thrown)?;

                    let pluralization = if sqls.len() == 1 {
                        "query was"
                    } else {
                        "queries were"
                    };

                    notification_ui.text(&[&format!(
                        "    <info>{}</info> {pluralization} executed",
                        sqls.len()
                    )])?;
                    notification_ui.success(&["Database schema updated successfully!"])?;
                }

                if dump_sql || force {
                    return Ok(0);
                }

                notification_ui.caution(&[
                    "This operation should not be executed in a production environment!",
                    "",
                    "Use the incremental update to detect changes during development and use",
                    "the SQL DDL provided to manually update your database in production.",
                ])?;

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
