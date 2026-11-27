//! Port of `Doctrine\ORM\Tools\Console\Command\ClearCache\QueryRegionCommand`:
//! clears a second-level cache query region.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/ClearCache/QueryRegionCommand.php

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
use crate::cache::DEFAULT_QUERY_REGION_NAME;
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `QueryRegionCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\ClearCache\\QueryRegionCommand";

const HELP: &str = r"The <info>%command.name%</info> command is meant to clear a second-level cache query region for an associated Entity Manager.
It is possible to delete/invalidate all query region, a specific query region or flushes the cache provider.

The execution type differ on how you execute the command.
If you want to invalidate all entries for the default query region this command would do the work:

<info>%command.name%</info>

To invalidate entries for a specific query region you should use :

<info>%command.name% my_region_name</info>

If you want to invalidate all entries for the all query region:

<info>%command.name% --all</info>

Alternatively, if you want to flush the configured cache provider use this command:

<info>%command.name% my_region_name --flush</info>

Finally, be aware that if <info>--flush</info> option is passed,
not all cache providers are able to flush entries, because of a limitation of its execution nature.";

/// `QueryRegionCommand`.
pub struct QueryRegionCommand(AbstractEntityManagerCommand);

impl QueryRegionCommand {
    /// `new QueryRegionCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for QueryRegionCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:clear-cache:region:query")?
            .set_description("Clear a second-level cache query region")
            .add_argument(
                "region-name",
                Some(OPTIONAL),
                "The query region to clear.",
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
                "If defined, all query regions will be deleted/invalidated.",
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
        let name = input.get_argument("region-name")?;
        let all = input.get_option("all")?.is_truthy();
        let flush = input.get_option("flush")?.is_truthy();
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;
        let cache = em.get_cache();

        let name = if name.is_null() {
            DEFAULT_QUERY_REGION_NAME.to_owned()
        } else {
            to_string(&name)
        };

        let Some(cache) = cache else {
            return Err(exception(
                "InvalidArgumentException",
                "No second-level cache is configured on the given EntityManager.",
            ));
        };

        if flush {
            cache
                .get_query_cache(Some(&name))
                .map_err(thrown)?
                .get_region()
                .evict_all()
                .map_err(thrown)?;

            ui.comment(&[&format!(
                "Flushing cache provider configured for second-level cache query region named <info>\"{name}\"</info>"
            )])?;

            return Ok(0);
        }

        if all {
            ui.comment(&["Clearing <info>all</info> second-level cache query regions"])?;

            cache.evict_query_regions().map_err(thrown)?;

            return Ok(0);
        }

        ui.comment(&[&format!(
            "Clearing second-level cache query region named <info>\"{name}\"</info>"
        )])?;
        cache.evict_query_region(Some(&name)).map_err(thrown)?;

        Ok(0)
    }
}
