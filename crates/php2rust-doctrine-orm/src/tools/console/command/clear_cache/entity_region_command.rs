//! Port of `Doctrine\ORM\Tools\Console\Command\ClearCache\EntityRegionCommand`:
//! clears a second-level cache entity region.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ClearCache/EntityRegionCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_argument::OPTIONAL;
use symfony_console::input::input_option::{Shortcut, VALUE_NONE, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::super::abstract_entity_manager_command::{
    AbstractEntityManagerCommand, exception, thrown, to_string,
};
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `EntityRegionCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\ClearCache\\EntityRegionCommand";

const HELP: &str = r"The <info>%command.name%</info> command is meant to clear a second-level cache entity region for an associated Entity Manager.
It is possible to delete/invalidate all entity region, a specific entity region or flushes the cache provider.

The execution type differ on how you execute the command.
If you want to invalidate all entries for an entity region this command would do the work:

<info>%command.name% 'Entities\MyEntity'</info>

To invalidate a specific entry you should use :

<info>%command.name% 'Entities\MyEntity' 1</info>

If you want to invalidate all entries for the all entity regions:

<info>%command.name% --all</info>

Alternatively, if you want to flush the configured cache provider for an entity region use this command:

<info>%command.name% 'Entities\MyEntity' --flush</info>

Finally, be aware that if <info>--flush</info> option is passed,
not all cache providers are able to flush entries, because of a limitation of its execution nature.";

/// `EntityRegionCommand`.
pub struct EntityRegionCommand(AbstractEntityManagerCommand);

impl EntityRegionCommand {
    /// `new EntityRegionCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for EntityRegionCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:clear-cache:region:entity")?
            .set_description("Clear a second-level cache entity region")
            .add_argument(
                "entity-class",
                Some(OPTIONAL),
                "The entity name.",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_argument(
                "entity-id",
                Some(OPTIONAL),
                "The entity identifier.",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_option(
                "em",
                Shortcut::None,
                Some(VALUE_REQUIRED),
                "Name of the entity manager to operate on",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_option(
                "all",
                Shortcut::None,
                Some(VALUE_NONE),
                "If defined, all entity regions will be deleted/invalidated.",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_option(
                "flush",
                Shortcut::None,
                Some(VALUE_NONE),
                "If defined, all cache entries will be flushed.",
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
        let entity_class = input.get_argument("entity-class")?;
        let entity_id = input.get_argument("entity-id")?;
        let all = input.get_option("all")?.is_truthy();
        let flush = input.get_option("flush")?.is_truthy();
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;
        let Some(cache) = em.get_cache() else {
            return Err(exception(
                "InvalidArgumentException",
                "No second-level cache is configured on the given EntityManager.",
            ));
        };

        if !entity_class.is_truthy() && !all {
            return Err(exception(
                "InvalidArgumentException",
                "Invalid argument \"--entity-class\"",
            ));
        }
        let entity_class = to_string(&entity_class);

        if flush {
            let region = cache
                .get_entity_cache_region(&entity_class)
                .map_err(thrown)?
                .ok_or_else(|| {
                    exception("Error", "Call to a member function evictAll() on null")
                })?;
            region.evict_all().map_err(thrown)?;

            ui.comment(&[&format!(
                "Flushing cache provider configured for entity named <info>\"{entity_class}\"</info>"
            )])?;

            return Ok(0);
        }

        if all {
            ui.comment(&["Clearing <info>all</info> second-level cache entity regions"])?;

            cache.evict_entity_regions().map_err(thrown)?;

            return Ok(0);
        }

        if entity_id.is_truthy() {
            ui.comment(&[&format!(
                "Clearing second-level cache entry for entity <info>\"{entity_class}\"</info> identified by <info>\"{}\"</info>",
                to_string(&entity_id)
            )])?;
            cache
                .evict_entity(&entity_class, &entity_id)
                .map_err(thrown)?;

            return Ok(0);
        }

        ui.comment(&[&format!(
            "Clearing second-level cache for entity <info>\"{entity_class}\"</info>"
        )])?;
        cache.evict_entity_region(&entity_class).map_err(thrown)?;

        Ok(0)
    }
}
