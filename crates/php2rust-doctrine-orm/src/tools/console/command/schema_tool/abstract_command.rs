//! Port of `Doctrine\ORM\Tools\Console\Command\SchemaTool\AbstractCommand`:
//! what the schema tool commands share. `executeSchemaCommand()` is the
//! closure `execute()` takes; it builds its own `SymfonyStyle`s on the
//! output (PHP passes the one `execute()` made, which has written nothing
//! by then).
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/SchemaTool/AbstractCommand.php

use std::sync::Arc;

use php2rust_doctrine_persistence::mapping::AbstractClassMetadataFactory;
use symfony_console::exception::Result;
use symfony_console::input::input::Input;
use symfony_console::output::output::OutputInterface;
use symfony_console::style::symfony_style::SymfonyStyle;

use crate::mapping::ClassMetadata;
use crate::tools::SchemaTool;
use crate::tools::console::command::abstract_entity_manager_command::{
    AbstractEntityManagerCommand, thrown,
};

/// `execute($input, $output)`: the metadata of every class, and
/// `executeSchemaCommand()` unless there is none.
pub fn execute(
    command: &AbstractEntityManagerCommand,
    input: &Input,
    output: &mut dyn OutputInterface,
    execute_schema_command: impl FnOnce(
        &Input,
        &mut dyn OutputInterface,
        &SchemaTool,
        &[Arc<ClassMetadata>],
    ) -> Result<i64>,
) -> Result<i64> {
    let em = command.get_entity_manager(input)?;

    let metadatas = em
        .get_metadata_factory()
        .get_all_metadata()
        .map_err(thrown)?;

    if metadatas.is_empty() {
        SymfonyStyle::new(input, output)?
            .get_error_style()?
            .success(&["No Metadata Classes to process."])?;

        return Ok(0);
    }

    execute_schema_command(input, output, &SchemaTool::new(&em), &metadatas)
}
