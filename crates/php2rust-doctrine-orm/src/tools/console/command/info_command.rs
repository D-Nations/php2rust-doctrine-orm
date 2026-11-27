//! Port of `Doctrine\ORM\Tools\Console\Command\InfoCommand`: shows basic
//! information about all mapped entities.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/InfoCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_option::{Shortcut, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::abstract_entity_manager_command::{AbstractEntityManagerCommand, exception, thrown};
use crate::mapping::mapping_exception::is_mapping_exception;
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `InfoCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\InfoCommand";

const HELP: &str = r"The <info>%command.name%</info> shows basic information about which
entities exist and possibly if their mapping information contains errors or
not.";

/// `InfoCommand`.
pub struct InfoCommand(AbstractEntityManagerCommand);

impl InfoCommand {
    /// `new InfoCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for InfoCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:info")?
            .set_description("Show basic information about all mapped entities")
            .add_option(
                "em",
                Shortcut::None,
                Some(VALUE_REQUIRED),
                "Name of the entity manager to operate on",
                Value::Null,
                SuggestedValues::None,
            )?
            .set_help(HELP);
        Ok(())
    }

    fn execute(
        &mut self,
        _command: &mut Command,
        input: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        let entity_manager = self.0.get_entity_manager(input)?;
        let mut ui = SymfonyStyle::new(input, output)?;

        let entity_class_names = entity_manager
            .get_configuration()
            .get_metadata_driver_impl()
            .ok_or_else(|| {
                exception(
                    "Error",
                    "Call to a member function getAllClassNames() on null",
                )
            })?
            .get_all_class_names()
            .map_err(thrown)?;

        if entity_class_names.is_empty() {
            ui.get_error_style()?.caution(&[
                "You do not have any mapped Doctrine ORM entities according to the current configuration.",
                "If you have entities or mapping files you should check your mapping configuration for errors.",
            ])?;

            return Ok(1);
        }

        ui.text(&[&format!(
            "Found <info>{}</info> mapped entities:",
            entity_class_names.len()
        )])?;
        ui.new_line(1)?;

        let mut failure = false;

        for entity_class_name in &entity_class_names {
            match entity_manager.get_class_metadata(entity_class_name) {
                Ok(_) => ui.text(&[&format!("<info>[OK]</info>   {entity_class_name}")])?,
                Err(e) if is_mapping_exception(&e) => {
                    ui.text(&[
                        &format!("<error>[FAIL]</error> {entity_class_name}"),
                        &format!("<comment>{}</comment>", e.message.to_string_lossy()),
                        "",
                    ])?;

                    failure = true;
                }
                Err(e) => return Err(thrown(e)),
            }
        }

        Ok(i64::from(failure))
    }
}
