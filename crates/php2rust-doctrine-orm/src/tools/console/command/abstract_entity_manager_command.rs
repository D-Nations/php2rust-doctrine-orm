//! Port of `Doctrine\ORM\Tools\Console\Command\AbstractEntityManagerCommand`:
//! what the ORM commands share. A command is a symfony-console
//! `CommandHandler` holding this; its `getEntityManager()` reads the
//! `--em` option.
//!
//! Also the glue between the ORM and the console: an ORM exception
//! (`Thrown`) leaves a command as a `ConsoleError::Other` of the same
//! class and message, and `block_on()` runs the ORM's async calls from a
//! command's synchronous `execute()`.
//!
//! Upstream: doctrine/orm src/Tools/Console/Command/AbstractEntityManagerCommand.php

use std::future::Future;
use std::sync::Arc;

use php_value::{Thrown, Value};
use symfony_console::exception::ConsoleError;
use symfony_console::input::input::Input;

use crate::entity_manager::EntityManager;
use crate::tools::console::entity_manager_provider::EntityManagerProvider;

/// `AbstractEntityManagerCommand`'s state: its entity manager provider.
#[derive(Clone)]
pub struct AbstractEntityManagerCommand {
    entity_manager_provider: Arc<dyn EntityManagerProvider>,
}

impl AbstractEntityManagerCommand {
    pub fn new(entity_manager_provider: Arc<dyn EntityManagerProvider>) -> Self {
        Self {
            entity_manager_provider,
        }
    }

    /// `getEntityManager($input)`.
    pub fn get_entity_manager(&self, input: &Input) -> Result<Arc<EntityManager>, ConsoleError> {
        let em = input.get_option("em")?;
        if em.is_null() {
            return self
                .entity_manager_provider
                .get_default_manager()
                .map_err(thrown);
        }
        self.entity_manager_provider
            .get_manager(&to_string(&em))
            .map_err(thrown)
    }

    /// The provider.
    pub fn entity_manager_provider(&self) -> &Arc<dyn EntityManagerProvider> {
        &self.entity_manager_provider
    }
}

/// An ORM exception leaving a command.
pub fn thrown(e: Thrown) -> ConsoleError {
    ConsoleError::Other {
        class: e.class,
        message: e.message.to_string_lossy().into_owned(),
    }
}

/// `new $class($message)` leaving a command.
pub fn exception(class: &str, message: impl Into<String>) -> ConsoleError {
    ConsoleError::Other {
        class: class.to_owned(),
        message: message.into(),
    }
}

/// `(string) $value` of an argument or option.
pub fn to_string(value: &Value) -> String {
    value
        .to_php_string()
        .map_or_default(|s| s.to_string_lossy().into_owned())
}

/// Runs one of the ORM's async calls from a command: on the current
/// Tokio runtime when there is one (it must be a multi-threaded runtime),
/// else on a new current-thread runtime.
pub fn block_on<F: Future>(future: F) -> Result<F::Output, ConsoleError> {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::CurrentThread {
            return Err(exception(
                "RuntimeException",
                "A console command cannot wait on a current-thread Tokio runtime; use a multi-threaded one",
            ));
        }
        return Ok(tokio::task::block_in_place(|| handle.block_on(future)));
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| exception("RuntimeException", e.to_string()))?;
    Ok(runtime.block_on(future))
}
