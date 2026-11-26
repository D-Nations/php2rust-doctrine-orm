//! Contract covering connection for a Doctrine persistence layer
//! `ManagerRegistry` class to implement.
//!
//! Upstream: doctrine/persistence src/ConnectionRegistry.php

use std::any::Any;
use std::sync::Arc;

use php_value::Thrown;

/// A connection: any object (a DBAL connection, downcast by the caller).
pub type Connection = Arc<dyn Any + Send + Sync>;

/// `Doctrine\Persistence\ConnectionRegistry`.
pub trait ConnectionRegistry: Send + Sync {
    /// Gets the default connection name.
    fn get_default_connection_name(&self) -> String;

    /// Gets the named connection (`None` for the default one).
    fn get_connection(&self, name: Option<&str>) -> Result<Connection, Thrown>;

    /// Gets all registered connections, by name.
    fn get_connections(&self) -> Result<Vec<(String, Connection)>, Thrown>;

    /// Gets all connection names (name → service id).
    fn get_connection_names(&self) -> Vec<(String, String)>;
}
