//! The `StaticPHPDriver` calls a static `loadMetadata()` method on your entity
//! classes where you can manually populate the `ClassMetadata` instance.
//!
//! The static method is the class's `ClassInfo::load_metadata`
//! (`php_reflection`).
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/StaticPHPDriver.php

use std::sync::Arc;

use php_value::Thrown;

use crate::mapping::ClassMetadata;
use crate::mapping::driver::{ClassLocator, ColocatedMappingDriver, MappingDriver};
use crate::php_reflection;

/// `Doctrine\Persistence\Mapping\Driver\StaticPHPDriver`.
pub struct StaticPHPDriver {
    colocated: ColocatedMappingDriver,
}

/// The constructor's argument: paths or a class locator.
pub enum StaticPHPDriverPaths {
    Paths(Vec<String>),
    Locator(Arc<dyn ClassLocator>),
}

impl StaticPHPDriver {
    pub const CLASS: &str = "Doctrine\\Persistence\\Mapping\\Driver\\StaticPHPDriver";

    pub fn new(paths: StaticPHPDriverPaths) -> Self {
        let colocated = ColocatedMappingDriver::new(Self::CLASS);
        match paths {
            StaticPHPDriverPaths::Locator(locator) => colocated.set_class_locator(locator),
            StaticPHPDriverPaths::Paths(paths) => colocated.add_paths(&paths),
        }
        Self { colocated }
    }
}

impl MappingDriver for StaticPHPDriver {
    fn load_metadata_for_class(
        &self,
        class_name: &str,
        metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        php_reflection::call_load_metadata(class_name, metadata).unwrap_or_else(|| {
            Err(Thrown::new(
                "Error",
                format!("Call to undefined method {class_name}::loadMetadata()"),
                0,
            ))
        })
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        self.colocated
            .get_all_class_names(|class_name| self.is_transient(class_name))
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        Ok(!php_reflection::method_exists(class_name, "loadMetadata"))
    }
}
