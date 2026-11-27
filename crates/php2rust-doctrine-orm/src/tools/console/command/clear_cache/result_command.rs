//! Port of `Doctrine\ORM\Tools\Console\Command\ClearCache\ResultCommand`:
//! clears the result cache of the entity manager.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ClearCache/ResultCommand.php

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

/// `ResultCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\ClearCache\\ResultCommand";

const HELP: &str = r"The <info>%command.name%</info> command is meant to clear the result cache of associated Entity Manager.
It is possible to invalidate all cache entries at once - called delete -, or flushes the cache provider
instance completely.

The execution type differ on how you execute the command.
If you want to invalidate the entries (and not delete from cache instance), this command would do the work:

<info>%command.name%</info>

Alternatively, if you want to flush the cache provider using this command:

<info>%command.name% --flush</info>

Finally, be aware that if <info>--flush</info> option is passed, not all cache providers are able to flush entries,
because of a limitation of its execution nature.";

/// `ResultCommand`.
pub struct ResultCommand(AbstractEntityManagerCommand);

impl ResultCommand {
    /// `new ResultCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for ResultCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:clear-cache:result")?
            .set_description("Clear all result cache of the various cache drivers")
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
        let Some(cache) = em.get_configuration().get_result_cache() else {
            return Err(exception(
                "InvalidArgumentException",
                "No Result cache driver is configured on given EntityManager.",
            ));
        };

        ui.comment(&["Clearing <info>all</info> Result cache entries"])?;

        let message = if cache.clear("") {
            "Successfully deleted cache entries."
        } else {
            "No cache entries were deleted."
        };

        ui.success(&[message])?;

        Ok(0)
    }
}
