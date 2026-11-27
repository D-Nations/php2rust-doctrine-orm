//! Port of `Doctrine\ORM\Tools\Console\Command\GenerateProxiesCommand`.
//! The port has only native lazy objects (PHP 8.4), so the command
//! triggers the deprecation, warns and does nothing, as upstream does on
//! PHP 8.4 with native lazy objects.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/GenerateProxiesCommand.php

use std::sync::Arc;

use php_value::Value;
use symfony_console::command::command::{Command, CommandHandler};
use symfony_console::exception::Result;
use symfony_console::input::SuggestedValues;
use symfony_console::input::input::Input;
use symfony_console::input::input_argument::OPTIONAL;
use symfony_console::input::input_option::{Shortcut, VALUE_IS_ARRAY, VALUE_REQUIRED};
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use super::abstract_entity_manager_command::{
    AbstractEntityManagerCommand, exception, thrown, to_string,
};
use crate::tools::console::entity_manager_provider::EntityManagerProvider;
use crate::tools::console::metadata_filter;
use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;

/// `GenerateProxiesCommand::class`.
pub const CLASS: &str = "Doctrine\\ORM\\Tools\\Console\\Command\\GenerateProxiesCommand";

/// The deprecation of generating proxies.
const DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12005";

/// `GenerateProxiesCommand`.
pub struct GenerateProxiesCommand(AbstractEntityManagerCommand);

impl GenerateProxiesCommand {
    /// `new GenerateProxiesCommand($entityManagerProvider)`: the command with this handler.
    pub fn command(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Result<Command> {
        Command::with_handler(
            Self(AbstractEntityManagerCommand::new(entity_manager_provider)),
            None,
        )
    }
}

impl CommandHandler for GenerateProxiesCommand {
    fn class(&self) -> &str {
        CLASS
    }

    fn configure(&mut self, command: &mut Command) -> Result<()> {
        command
            .set_name("orm:generate-proxies")?
            .set_aliases(["orm:generate:proxies"])?
            .set_description("Generates proxy classes for entity classes")
            .add_argument(
                "dest-path",
                Some(OPTIONAL),
                "The path to generate your proxy classes. If none is provided, it will attempt to grab from configuration.",
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
                "filter",
                Shortcut::None,
                Some(VALUE_REQUIRED | VALUE_IS_ARRAY),
                "A string pattern used to match entities that should be processed.",
                Value::Null,
                SuggestedValues::None,
            )?
            .set_help("Generates proxy classes for entity classes.");
        Ok(())
    }

    fn execute(
        &mut self,
        command: &mut Command,
        input: &mut Input,
        output: &mut dyn OutputInterface,
    ) -> Result<i64> {
        let em = self.0.get_entity_manager(input)?;
        let filter: Vec<String> = match input.get_option("filter")? {
            Value::Array(filter) => filter.iter().map(|(_, f)| to_string(f)).collect(),
            other if other.is_null() => Vec::new(),
            other => vec![to_string(&other)],
        };
        let dest_path = input.get_argument("dest-path")?;
        let mut ui = SymfonyStyle::new(input, output)?.get_error_style()?;

        // PHP_VERSION_ID >= 80400
        crate::deprecations::trigger(DEPRECATION, || {
            "Generating proxies is deprecated and will be impossible in Doctrine ORM 4.0."
                .to_owned()
        });

        if em.get_configuration().is_native_lazy_objects_enabled() {
            let name = command.get_name().unwrap_or_default();

            ui.warning(&[&format!(
                "When using native lazy objects, you do not need to generate proxy classes.\n\
                 Attempting to do so is a no-op, and {name} will be removed in Doctrine ORM 4.0."
            )])?;

            return Ok(0);
        }

        let metadatas = em
            .get_metadata_factory()
            .get_all_metadata()
            .map_err(thrown)?;
        let metadatas = metadata_filter::filter(metadatas, &filter).map_err(thrown)?;

        // Process destination directory
        let dest_path = if dest_path.is_null() {
            em.get_configuration()
                .get_proxy_dir()
                .map(str::to_owned)
                .ok_or_else(|| {
                    exception("InvalidArgumentException", "Proxy directory cannot be null")
                })?
        } else {
            to_string(&dest_path)
        };

        if !std::path::Path::new(&dest_path).is_dir() {
            let _ = std::fs::create_dir_all(&dest_path);
        }

        let Ok(dest_path) = std::fs::canonicalize(&dest_path) else {
            return Err(exception(
                "InvalidArgumentException",
                format!(
                    "Proxies destination directory '<info>{}</info>' does not exist.",
                    em.get_configuration().get_proxy_dir().unwrap_or_default()
                ),
            ));
        };
        let dest_path = dest_path.to_string_lossy().into_owned();

        if std::fs::metadata(&dest_path).is_ok_and(|m| m.permissions().readonly()) {
            return Err(exception(
                "InvalidArgumentException",
                format!(
                    "Proxies destination directory '<info>{dest_path}</info>' does not have write permissions."
                ),
            ));
        }

        if metadatas.is_empty() {
            ui.success(&["No Metadata Classes to process."])?;

            return Ok(0);
        }

        for metadata in &metadatas {
            ui.text(&[&format!(
                "Processing entity \"<info>{}</info>\"",
                metadata.read().name
            )])?;
        }

        // Generating Proxies
        em.get_proxy_factory()
            .generate_proxy_classes(&metadatas, Some(&dest_path));

        // Outputting information message
        ui.new_line(1)?;
        ui.text(&[&format!(
            "Proxy classes generated to \"<info>{dest_path}</info>\""
        )])?;

        Ok(0)
    }
}
