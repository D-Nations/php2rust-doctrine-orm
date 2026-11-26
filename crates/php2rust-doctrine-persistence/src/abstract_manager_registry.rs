//! Abstract implementation of the `ManagerRegistry` contract.
//!
//! The abstract class is the [`AbstractManagerRegistry`] trait: an
//! implementation holds a [`ManagerRegistryState`] (the constructor's
//! arguments) and implements `getService()` and `resetService()`; the
//! [`ManagerRegistry`] and [`ConnectionRegistry`] methods come with it.
//!
//! Upstream: doctrine/persistence src/AbstractManagerRegistry.php

use std::sync::Arc;

use php_value::Thrown;

use crate::php_reflection::ReflectionClass;
use crate::{Connection, ConnectionRegistry, ManagerRegistry, ObjectManager, ObjectRepository};

/// A service `getService()` returns: a connection or an object manager.
#[derive(Clone)]
pub enum Service {
    Connection(Connection),
    Manager(Arc<dyn ObjectManager>),
}

impl Service {
    /// The service as `getConnection()` returns it (any object).
    pub fn into_connection(self) -> Connection {
        match self {
            Self::Connection(connection) => connection,
            Self::Manager(manager) => Arc::new(manager),
        }
    }

    /// `assert($service instanceof ObjectManager)`.
    fn into_manager(self) -> Result<Arc<dyn ObjectManager>, Thrown> {
        match self {
            Self::Manager(manager) => Ok(manager),
            Self::Connection(_) => Err(Thrown::new(
                "AssertionError",
                "assert($service instanceof ObjectManager)",
                1,
            )),
        }
    }
}

/// The constructor's arguments.
#[derive(Debug, Clone)]
pub struct ManagerRegistryState {
    name: String,
    connections: Vec<(String, String)>,
    managers: Vec<(String, String)>,
    default_connection: String,
    default_manager: String,
    proxy_interface_name: Option<String>,
}

impl ManagerRegistryState {
    /// `connections` and `managers` map names to service ids.
    pub fn new(
        name: impl Into<String>,
        connections: Vec<(String, String)>,
        managers: Vec<(String, String)>,
        default_connection: impl Into<String>,
        default_manager: impl Into<String>,
        proxy_interface_name: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            connections,
            managers,
            default_connection: default_connection.into(),
            default_manager: default_manager.into(),
            proxy_interface_name,
        }
    }
}

fn lookup<'a>(map: &'a [(String, String)], name: &str) -> Option<&'a str> {
    map.iter()
        .find(|(key, _)| key == name)
        .map(|(_, id)| id.as_str())
}

fn invalid_argument(message: String) -> Thrown {
    Thrown::new("InvalidArgumentException", message, 0)
}

/// `Doctrine\Persistence\AbstractManagerRegistry`.
pub trait AbstractManagerRegistry: Send + Sync {
    fn registry_state(&self) -> &ManagerRegistryState;

    /// Fetches/creates the given services.
    ///
    /// A service in this context is connection or a manager instance.
    fn get_service(&self, name: &str) -> Result<Service, Thrown>;

    /// Resets the given services.
    ///
    /// A service in this context is connection or a manager instance.
    fn reset_service(&self, name: &str);

    /// Gets the name of the registry.
    fn get_name(&self) -> &str {
        &self.registry_state().name
    }
}

/// `selectManager()`.
fn select_manager<T: AbstractManagerRegistry + ?Sized>(
    registry: &T,
    persistent_object: &str,
    persistent_manager_name: Option<&str>,
) -> Result<Arc<dyn ObjectManager>, Thrown> {
    if persistent_manager_name.is_some() {
        return get_manager(registry, persistent_manager_name);
    }
    match get_manager_for_class(registry, persistent_object)? {
        Some(manager) => Ok(manager),
        None => get_manager(registry, None),
    }
}

fn get_manager<T: AbstractManagerRegistry + ?Sized>(
    registry: &T,
    name: Option<&str>,
) -> Result<Arc<dyn ObjectManager>, Thrown> {
    let state = registry.registry_state();
    let name = name.unwrap_or(&state.default_manager);
    let Some(id) = lookup(&state.managers, name) else {
        return Err(invalid_argument(format!(
            "Doctrine {} Manager named \"{name}\" does not exist.",
            state.name
        )));
    };
    registry.get_service(id)?.into_manager()
}

fn get_manager_for_class<T: AbstractManagerRegistry + ?Sized>(
    registry: &T,
    class: &str,
) -> Result<Option<Arc<dyn ObjectManager>>, Thrown> {
    let state = registry.registry_state();
    let mut class = class.to_owned();
    if let Some(proxy_interface_name) = &state.proxy_interface_name {
        let proxy_class = ReflectionClass::new(&class)?;
        if proxy_class.is_anonymous() {
            return Ok(None);
        }
        if proxy_class.implements_interface(proxy_interface_name)? {
            let Some(parent_class) = proxy_class.get_parent_class() else {
                return Ok(None);
            };
            parent_class.get_name().clone_into(&mut class);
        }
    }
    for (_, id) in &state.managers {
        let manager = registry.get_service(id)?.into_manager()?;
        if !manager.get_metadata_factory().is_transient(&class)? {
            return Ok(Some(manager));
        }
    }
    Ok(None)
}

impl<T: AbstractManagerRegistry> ConnectionRegistry for T {
    fn get_default_connection_name(&self) -> String {
        self.registry_state().default_connection.clone()
    }

    fn get_connection(&self, name: Option<&str>) -> Result<Connection, Thrown> {
        let state = self.registry_state();
        let name = name.unwrap_or(&state.default_connection);
        let Some(id) = lookup(&state.connections, name) else {
            return Err(invalid_argument(format!(
                "Doctrine {} Connection named \"{name}\" does not exist.",
                state.name
            )));
        };
        Ok(self.get_service(id)?.into_connection())
    }

    fn get_connections(&self) -> Result<Vec<(String, Connection)>, Thrown> {
        let mut connections = Vec::new();
        for (name, id) in &self.registry_state().connections {
            connections.push((name.clone(), self.get_service(id)?.into_connection()));
        }
        Ok(connections)
    }

    fn get_connection_names(&self) -> Vec<(String, String)> {
        self.registry_state().connections.clone()
    }
}

impl<T: AbstractManagerRegistry> ManagerRegistry for T {
    fn get_default_manager_name(&self) -> String {
        self.registry_state().default_manager.clone()
    }

    fn get_manager(&self, name: Option<&str>) -> Result<Arc<dyn ObjectManager>, Thrown> {
        get_manager(self, name)
    }

    fn get_managers(&self) -> Result<crate::manager_registry::Managers, Thrown> {
        let mut managers = Vec::new();
        for (name, id) in &self.registry_state().managers {
            managers.push((name.clone(), self.get_service(id)?.into_manager()?));
        }
        Ok(managers)
    }

    fn reset_manager(&self, name: Option<&str>) -> Result<Arc<dyn ObjectManager>, Thrown> {
        let state = self.registry_state();
        let name = name.unwrap_or(&state.default_manager);
        let Some(id) = lookup(&state.managers, name) else {
            return Err(invalid_argument(format!(
                "Doctrine {} Manager named \"{name}\" does not exist.",
                state.name
            )));
        };
        // force the creation of a new document manager
        // if the current one is closed
        self.reset_service(id);
        get_manager(self, Some(name))
    }

    fn get_manager_names(&self) -> Vec<(String, String)> {
        self.registry_state().managers.clone()
    }

    fn get_repository(
        &self,
        persistent_object: &str,
        persistent_manager_name: Option<&str>,
    ) -> Result<Arc<dyn ObjectRepository>, Thrown> {
        select_manager(self, persistent_object, persistent_manager_name)?
            .get_repository(persistent_object)
    }

    fn get_manager_for_class(&self, class: &str) -> Result<Option<Arc<dyn ObjectManager>>, Thrown> {
        get_manager_for_class(self, class)
    }
}
