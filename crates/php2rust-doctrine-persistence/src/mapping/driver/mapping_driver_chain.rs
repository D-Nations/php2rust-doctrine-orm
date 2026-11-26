//! The `DriverChain` allows you to add multiple other mapping drivers for
//! certain namespaces.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/MappingDriverChain.php

use std::collections::HashSet;
use std::sync::{Arc, PoisonError, RwLock};

use php_value::Thrown;

use crate::mapping::driver::MappingDriver;
use crate::mapping::{ClassMetadata, mapping_exception};

/// `Doctrine\Persistence\Mapping\Driver\MappingDriverChain`.
#[derive(Default)]
pub struct MappingDriverChain {
    /// The default driver.
    default_driver: RwLock<Option<Arc<dyn MappingDriver>>>,
    /// Namespace → driver, in the order they were first added.
    drivers: RwLock<Vec<(String, Arc<dyn MappingDriver>)>>,
}

impl MappingDriverChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the default driver.
    pub fn get_default_driver(&self) -> Option<Arc<dyn MappingDriver>> {
        self.default_driver
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Set the default driver.
    pub fn set_default_driver(&self, driver: Arc<dyn MappingDriver>) {
        *self
            .default_driver
            .write()
            .unwrap_or_else(PoisonError::into_inner) = Some(driver);
    }

    /// Adds a nested driver.
    pub fn add_driver(&self, nested_driver: Arc<dyn MappingDriver>, namespace: &str) {
        let mut drivers = self.drivers.write().unwrap_or_else(PoisonError::into_inner);
        match drivers.iter_mut().find(|(ns, _)| ns == namespace) {
            Some(entry) => entry.1 = nested_driver,
            None => drivers.push((namespace.to_owned(), nested_driver)),
        }
    }

    /// Gets the array of nested drivers, by namespace.
    pub fn get_drivers(&self) -> Vec<(String, Arc<dyn MappingDriver>)> {
        self.drivers
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn driver_for(&self, class_name: &str) -> Option<Arc<dyn MappingDriver>> {
        self.get_drivers()
            .into_iter()
            .find(|(namespace, _)| class_name.starts_with(namespace.as_str()))
            .map(|(_, driver)| driver)
    }
}

impl MappingDriver for MappingDriverChain {
    fn load_metadata_for_class(
        &self,
        class_name: &str,
        metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        if let Some(driver) = self.driver_for(class_name) {
            return driver.load_metadata_for_class(class_name, metadata);
        }

        if let Some(default_driver) = self.get_default_driver() {
            return default_driver.load_metadata_for_class(class_name, metadata);
        }

        let namespaces: Vec<String> = self.get_drivers().into_iter().map(|(ns, _)| ns).collect();
        Err(mapping_exception::class_not_found_in_namespaces(
            class_name,
            &namespaces,
        ))
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        let mut class_names: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut driver_classes: Vec<(*const (), Vec<String>)> = Vec::new();

        for (namespace, driver) in self.get_drivers() {
            let oid = Arc::as_ptr(&driver).cast::<()>();
            if !driver_classes.iter().any(|(id, _)| *id == oid) {
                driver_classes.push((oid, driver.get_all_class_names()?));
            }
            let Some((_, classes)) = driver_classes.iter().find(|(id, _)| *id == oid) else {
                continue;
            };
            for class_name in classes {
                if !class_name.starts_with(namespace.as_str()) {
                    continue;
                }
                if seen.insert(class_name.clone()) {
                    class_names.push(class_name.clone());
                }
            }
        }

        if let Some(default_driver) = self.get_default_driver() {
            for class_name in default_driver.get_all_class_names()? {
                if seen.insert(class_name.clone()) {
                    class_names.push(class_name);
                }
            }
        }

        Ok(class_names)
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        if let Some(driver) = self.driver_for(class_name) {
            return driver.is_transient(class_name);
        }

        if let Some(default_driver) = self.get_default_driver() {
            return default_driver.is_transient(class_name);
        }

        Ok(true)
    }
}
