//! Repository factories: the interface for entity repository factories
//! and the factory used to create default repository instances.
//!
//! A repository class other than `Doctrine\ORM\EntityRepository` must be a
//! registered class extending it; its repository is an
//! [`EntityRepository`] that reports that class name (PHP instantiates the
//! subclass, whose own methods are not ported).
//!
//! Upstream: doctrine/orm src/Repository/RepositoryFactory.php src/Repository/DefaultRepositoryFactory.php

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use php_value::Thrown;
use php2rust_doctrine_persistence::php_reflection;

use crate::configuration::ENTITY_REPOSITORY_CLASS;
use crate::entity_manager::EntityManager;
use crate::entity_repository::EntityRepository;

/// `Doctrine\ORM\Repository\RepositoryFactory`.
pub trait RepositoryFactory: Send + Sync {
    /// Gets the repository for an entity class.
    fn get_repository(
        &self,
        entity_manager: &Arc<EntityManager>,
        entity_name: &str,
    ) -> Result<Arc<EntityRepository>, Thrown>;
}

/// `Doctrine\ORM\Repository\DefaultRepositoryFactory`.
#[derive(Default)]
pub struct DefaultRepositoryFactory {
    /// The list of `EntityRepository` instances.
    repository_list: Mutex<HashMap<String, Arc<EntityRepository>>>,
}

impl fmt::Debug for DefaultRepositoryFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultRepositoryFactory")
            .finish_non_exhaustive()
    }
}

impl DefaultRepositoryFactory {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_repository(
        entity_manager: &Arc<EntityManager>,
        entity_name: &str,
    ) -> Result<EntityRepository, Thrown> {
        let metadata = entity_manager.get_class_metadata(entity_name)?;
        let repository_class_name = metadata
            .read()
            .custom_repository_class_name
            .clone()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| {
                entity_manager
                    .get_configuration()
                    .get_default_repository_class_name()
            });
        if !repository_class_name.eq_ignore_ascii_case(ENTITY_REPOSITORY_CLASS) {
            if !php_reflection::class_exists(&repository_class_name) {
                return Err(Thrown::new(
                    "Error",
                    format!("Class \"{repository_class_name}\" not found"),
                    0,
                ));
            }
            if !php_reflection::is_a(&repository_class_name, ENTITY_REPOSITORY_CLASS) {
                return Err(Thrown::new(
                    "TypeError",
                    format!(
                        "Doctrine\\ORM\\Repository\\DefaultRepositoryFactory::createRepository(): Return value must be of type Doctrine\\ORM\\EntityRepository, {repository_class_name} returned"
                    ),
                    0,
                ));
            }
        }
        Ok(EntityRepository::with_class(
            entity_manager,
            metadata,
            repository_class_name,
        ))
    }
}

impl RepositoryFactory for DefaultRepositoryFactory {
    fn get_repository(
        &self,
        entity_manager: &Arc<EntityManager>,
        entity_name: &str,
    ) -> Result<Arc<EntityRepository>, Thrown> {
        let name = entity_manager
            .get_class_metadata(entity_name)?
            .read()
            .get_name()
            .to_owned();
        let repository_hash = format!("{name}{}", Arc::as_ptr(entity_manager) as usize);
        if let Some(repository) = self
            .repository_list
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&repository_hash)
            && repository.is_bound()
        {
            return Ok(repository.clone());
        }
        let repository = Arc::new(Self::create_repository(entity_manager, entity_name)?);
        self.repository_list
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(repository_hash, repository.clone());
        Ok(repository)
    }
}
