//! Port of `Doctrine\ORM\Tools\Console\Command\ClearCache\MetadataCommand`:
//! clears the metadata cache of the entity manager.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ClearCache/MetadataCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_option::{Shortcut, VALUE_NONE, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::super::abstract_entity_manager_command::{AbstractEntityManagerCommand, exception};
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `MetadataCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\ClearCache\\MetadataCommand";

const HELP: &str = r"The <info>%command.name%</info> command is meant to clear the metadata cache of associated Entity Manager.";

/// `MetadataCommand`.
pub struct MetadataCommand(AbstractEntityManagerCommand);

impl MetadataCommand {
    /// `new MetadataCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for MetadataCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:clear-cache:metadata")?
            .set_description("Clear all metadata cache of the various cache drivers")
            .add_option(
                "em",
                Shortcut::None,
                Some(VALUE_REQUIRED),
                "Name of the entity manager to operate on",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_option(
                "flush",
                Shortcut::None,
                Some(VALUE_NONE),
                "If defined, cache entries will be flushed instead of deleted/invalidated.",
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
        let em = self.0.get_entity_manager(input)?;
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;
        let Some(cache) = em.get_configuration().get_metadata_cache() else {
            return Err(exception(
                "InvalidArgumentException",
                "No Metadata cache driver is configured on given EntityManager.",
            ));
        };

        ui.comment(&["Clearing <info>all</info> Metadata cache entries"])?;

        let message = if cache.clear("") {
            "Successfully deleted cache entries."
        } else {
            "No cache entries were deleted."
        };

        ui.success(&[message])?;

        Ok(0)
    }
}
