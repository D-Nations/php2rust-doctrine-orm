//! Port of `Doctrine\ORM\Tools\Console\Command\ClearCache\CollectionRegionCommand`:
//! clears a second-level cache collection region.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ClearCache/CollectionRegionCommand.php

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

/// `CollectionRegionCommand::class`.
pub const CLASS: &str =
    "Doctrine\\ORM\\Tools\\Console\\Command\\ClearCache\\CollectionRegionCommand";

const HELP: &str = r"The <info>%command.name%</info> command is meant to clear a second-level cache collection regions for an associated Entity Manager.
It is possible to delete/invalidate all collection region, a specific collection region or flushes the cache provider.

The execution type differ on how you execute the command.
If you want to invalidate all entries for an collection region this command would do the work:

<info>%command.name% 'Entities\MyEntity' 'collectionName'</info>

To invalidate a specific entry you should use :

<info>%command.name% 'Entities\MyEntity' 'collectionName' 1</info>

If you want to invalidate all entries for the all collection regions:

<info>%command.name% --all</info>

Alternatively, if you want to flush the configured cache provider for an collection region use this command:

<info>%command.name% 'Entities\MyEntity' 'collectionName' --flush</info>

Finally, be aware that if <info>--flush</info> option is passed,
not all cache providers are able to flush entries, because of a limitation of its execution nature.";

/// `CollectionRegionCommand`.
pub struct CollectionRegionCommand(AbstractEntityManagerCommand);

impl CollectionRegionCommand {
    /// `new CollectionRegionCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for CollectionRegionCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:clear-cache:region:collection")?
            .set_description("Clear a second-level cache collection region")
            .add_argument(
                "owner-class",
                Some(OPTIONAL),
                "The owner entity name.",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_argument(
                "association",
                Some(OPTIONAL),
                "The association collection name.",
                Value::Null,
                SuggestedValues::None,
            )?
            .add_argument(
                "owner-id",
                Some(OPTIONAL),
                "The owner identifier.",
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
        let owner_class = input.get_argument("owner-class")?;
        let assoc = input.get_argument("association")?;
        let owner_id = input.get_argument("owner-id")?;
        let all = input.get_option("all")?.is_truthy();
        let flush = input.get_option("flush")?.is_truthy();
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;
        let Some(cache) = em.get_cache() else {
            return Err(exception(
                "InvalidArgumentException",
                "No second-level cache is configured on the given EntityManager.",
            ));
        };

        if (!owner_class.is_truthy() || !assoc.is_truthy()) && !all {
            return Err(exception(
                "InvalidArgumentException",
                "Missing arguments \"--owner-class\" \"--association\"",
            ));
        }
        let owner_class = to_string(&owner_class);
        let assoc = to_string(&assoc);

        if flush {
            let region = cache
                .get_collection_cache_region(&owner_class, &assoc)
                .map_err(thrown)?
                .ok_or_else(|| {
                    exception("Error", "Call to a member function evictAll() on null")
                })?;
            region.evict_all().map_err(thrown)?;

            ui.comment(&[&format!(
                "Flushing cache provider configured for <info>\"{owner_class}#{assoc}\"</info>"
            )])?;

            return Ok(0);
        }

        if all {
            ui.comment(&["Clearing <info>all</info> second-level cache collection regions"])?;

            cache.evict_entity_regions().map_err(thrown)?;

            return Ok(0);
        }

        if owner_id.is_truthy() {
            ui.comment(&[&format!(
                "Clearing second-level cache entry for collection <info>\"{owner_class}#{assoc}\"</info> owner entity identified by <info>\"{}\"</info>",
                to_string(&owner_id)
            )])?;
            cache
                .evict_collection(&owner_class, &assoc, &owner_id)
                .map_err(thrown)?;

            return Ok(0);
        }

        ui.comment(&[&format!(
            "Clearing second-level cache for collection <info>\"{owner_class}#{assoc}\"</info>"
        )])?;
        cache
            .evict_collection_region(&owner_class, &assoc)
            .map_err(thrown)?;

        Ok(0)
    }
}
