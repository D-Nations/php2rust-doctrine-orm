//! Port of `Doctrine\ORM\Tools\Console\Command\ValidateSchemaCommand`:
//! validates the mapping and checks it is in sync with the database.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ValidateSchemaCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_option::{Shortcut, VALUE_NONE, VALUE_REQUIRED};
use symfony_console::output::output::{OutputInterface, VERBOSITY_VERBOSE};
use symfony_console::style::symfony_style::SymfonyStyle;

use super::abstract_entity_manager_command::{AbstractEntityManagerCommand, block_on, thrown};
use crate::tools::console::entity_manager_provider::EntityManagerProvider;
use crate::tools::schema_validator::SchemaValidator;

/// `ValidateSchemaCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\ValidateSchemaCommand";

/// `ValidateSchemaCommand`.
pub struct ValidateSchemaCommand(AbstractEntityManagerCommand);

impl ValidateSchemaCommand {
    /// `new ValidateSchemaCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for ValidateSchemaCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        let option = |command: &mut Command, name: &str, mode: i64, description: &str| {
            command
                .add_option(
                    name,
                    Shortcut::None,
                    Some(mode),
                    description,
                    Value::Null,
                    SuggestedValues::None,
                )
                .map(|_| ())
        };
        command
            .set_name("orm:validate-schema")?
            .set_description("Validate the mapping files");
        option(
            command,
            "em",
            VALUE_REQUIRED,
            "Name of the entity manager to operate on",
        )?;
        option(
            command,
            "skip-mapping",
            VALUE_NONE,
            "Skip the mapping validation check",
        )?;
        option(
            command,
            "skip-sync",
            VALUE_NONE,
            "Skip checking if the mapping is in sync with the database",
        )?;
        option(
            command,
            "skip-property-types",
            VALUE_NONE,
            "Skip checking if property types match the Doctrine types",
        )?;
        command
            .set_help("Validate that the mapping files are correct and in sync with the database.");
        Ok(())
    }

    fn execute(
        &mut self,
        _command: &mut Command,
        input: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        let verbosity = output.get_verbosity();
        let em = self.0.get_entity_manager(input)?;
        let validator =
            SchemaValidator::new(&em, !input.get_option("skip-property-types")?.is_truthy());
        let skip_mapping = input.get_option("skip-mapping")?.is_truthy();
        let skip_sync = input.get_option("skip-sync")?.is_truthy();
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;
        let mut exit = 0;

        ui.section("Mapping")?;

        if skip_mapping {
            ui.text(&["<comment>[SKIPPED] The mapping was not checked.</comment>"])?;
        } else {
            let errors = validator.validate_mapping().map_err(thrown)?;
            if errors.is_empty() {
                ui.success(&["The mapping files are correct."])?;
            } else {
                for (class_name, error_messages) in &errors {
                    ui.text(&[&format!(
                        "<error>[FAIL]</error> The entity-class <comment>{class_name}</comment> mapping is invalid:"
                    )])?;

                    let error_messages: Vec<&str> =
                        error_messages.iter().map(String::as_str).collect();
                    ui.listing(&error_messages)?;
                    ui.new_line(1)?;
                }

                exit += 1;
            }
        }

        ui.section("Database")?;

        if skip_sync {
            ui.text(&[
                "<comment>[SKIPPED] The database was not checked for synchronicity.</comment>",
            ])?;
        } else if !block_on(validator.schema_in_sync_with_metadata())?.map_err(thrown)? {
            ui.error(&["The database schema is not in sync with the current mapping file."])?;

            if verbosity >= VERBOSITY_VERBOSE {
                let sqls = block_on(validator.get_update_schema_list())?.map_err(thrown)?;
                ui.comment(&[&format!(
                    "<info>{}</info> schema diff(s) detected:",
                    sqls.len()
                )])?;
                for sql in &sqls {
                    ui.text(&[&format!("    {sql};")])?;
                }
            }

            exit += 2;
        } else {
            ui.success(&["The database schema is in sync with the mapping files."])?;
        }

        Ok(exit)
    }
}
