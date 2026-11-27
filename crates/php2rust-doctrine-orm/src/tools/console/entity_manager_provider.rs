//! Port of `Doctrine\ORM\Tools\Console\EntityManagerProvider`,
//! `EntityManagerProvider\SingleManagerProvider` and
//! `EntityManagerProvider\UnknownManagerException`.
//!
//! Upstream: doctrine/orm src/Tools/Console/EntityManagerProvider.php src/Tools/Console/EntityManagerProvider/SingleManagerProvider.php src/Tools/Console/EntityManagerProvider/UnknownManagerException.php

use std::sync::Arc;

use php_value::Thrown;

use crate::entity_manager::EntityManager;

/// `UnknownManagerException::class`.
pub const UNKNOWN_MANAGER_EXCEPTION: &str =
    "Doctrine\\ORM\\Tools\\Console\\EntityManagerProvider\\UnknownManagerException";

/// `EntityManagerProvider`.
pub trait EntityManagerProvider: Send + Sync {
    /// `getDefaultManager()`.
    fn get_default_manager(&self) -> Result<Arc<EntityManager>, Thrown>;

    /// `getManager($name)`.
    fn get_manager(&self, name: &str) -> Result<Arc<EntityManager>, Thrown>;
}

/// `SingleManagerProvider`.
pub struct SingleManagerProvider {
    entity_manager: Arc<EntityManager>,
    default_manager_name: String,
}

impl SingleManagerProvider {
    /// `new SingleManagerProvider($entityManager)`.
    pub fn new(entity_manager: Arc<EntityManager>) -> Self {
        Self::with_name(entity_manager, "default")
    }

    /// `new SingleManagerProvider($entityManager, $defaultManagerName)`.
    pub fn with_name(entity_manager: Arc<EntityManager>, default_manager_name: &str) -> Self {
        Self {
            entity_manager,
            default_manager_name: default_manager_name.to_owned(),
        }
    }
}

impl EntityManagerProvider for SingleManagerProvider {
    fn get_default_manager(&self) -> Result<Arc<EntityManager>, Thrown> {
        Ok(Arc::clone(&self.entity_manager))
    }

    fn get_manager(&self, name: &str) -> Result<Arc<EntityManager>, Thrown> {
        if name != self.default_manager_name {
            return Err(unknown_manager(
                name,
                std::slice::from_ref(&self.default_manager_name),
            ));
        }

        Ok(Arc::clone(&self.entity_manager))
    }
}

/// `UnknownManagerException::unknownManager($unknownManager, $knownManagers)`.
pub fn unknown_manager(unknown_manager: &str, known_managers: &[String]) -> Thrown {
    Thrown::new(
        UNKNOWN_MANAGER_EXCEPTION,
        format!(
            "Requested unknown entity manager: {unknown_manager}, known managers: {}",
            known_managers.join(", ")
        ),
        0,
    )
}
