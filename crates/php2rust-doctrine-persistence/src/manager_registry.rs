//! Gives access to the object managers and connections of an application,
//! by name or by class.
//!
//! Upstream: doctrine/persistence src/ManagerRegistry.php

use std::sync::Arc;

use php_value::Thrown;

use crate::{ConnectionRegistry, ObjectManager, ObjectRepository};

/// The managers by name.
pub type Managers = Vec<(String, Arc<dyn ObjectManager>)>;

/// `Doctrine\Persistence\ManagerRegistry`.
pub trait ManagerRegistry: ConnectionRegistry {
    /// Gets the default object manager name.
    fn get_default_manager_name(&self) -> String;

    /// Gets a named object manager (`None` for the default one).
    fn get_manager(&self, name: Option<&str>) -> Result<Arc<dyn ObjectManager>, Thrown>;

    /// Gets all registered object managers, by name.
    fn get_managers(&self) -> Result<Managers, Thrown>;

    /// Resets a named object manager (`None` for the default one).
    ///
    /// This method is useful when an object manager has been closed
    /// because of a rollbacked transaction AND when you think that it makes
    /// sense to get a new one to replace the closed one.
    ///
    /// Be warned that you will get a brand new object manager as the
    /// existing one is not useable anymore. This means that any other
    /// object with a dependency on this object manager will hold an
    /// obsolete reference. You can inject the registry instead to avoid
    /// this problem.
    fn reset_manager(&self, name: Option<&str>) -> Result<Arc<dyn ObjectManager>, Thrown>;

    /// Gets all object manager names and associated service IDs. A service
    /// ID is a string that allows to obtain an object manager, typically
    /// from a PSR-11 container.
    fn get_manager_names(&self) -> Vec<(String, String)>;

    /// Gets the `ObjectRepository` for a persistent object, from the named
    /// object manager (`None` for the default one).
    fn get_repository(
        &self,
        persistent_object: &str,
        persistent_manager_name: Option<&str>,
    ) -> Result<Arc<dyn ObjectRepository>, Thrown>;

    /// Gets the object manager associated with a given class.
    fn get_manager_for_class(&self, class: &str) -> Result<Option<Arc<dyn ObjectManager>>, Thrown>;
}
