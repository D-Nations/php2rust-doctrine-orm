//! Port of `Doctrine\ORM\Tools\Console\Command\ClearCache\QueryCommand`:
//! clears the query cache of the entity manager. The port has no `APCu` adapter, so the
//! `ApcuAdapter` refusal is not reproduced.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ClearCache/QueryCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_option::{Shortcut, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::super::abstract_entity_manager_command::{AbstractEntityManagerCommand, exception};
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `QueryCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\ClearCache\\QueryCommand";

const HELP: &str = "The <info>%command.name%</info> command is meant to clear the query cache of associated Entity Manager.";

/// `QueryCommand`.
pub struct QueryCommand(AbstractEntityManagerCommand);

impl QueryCommand {
    /// `new QueryCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for QueryCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:clear-cache:query")?
            .set_description("Clear all query cache of the various cache drivers")
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
        let em = self.0.get_entity_manager(input)?;
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;
        let Some(cache) = em.get_configuration().get_query_cache() else {
            return Err(exception(
                "InvalidArgumentException",
                "No Query cache driver is configured on given EntityManager.",
            ));
        };

        ui.comment(&["Clearing <info>all</info> Query cache entries"])?;

        let message = if cache.clear("") {
            "Successfully deleted cache entries."
        } else {
            "No cache entries were deleted."
        };

        ui.success(&[message])?;

        Ok(0)
    }
}
