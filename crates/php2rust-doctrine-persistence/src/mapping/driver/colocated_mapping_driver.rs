//! The `ColocatedMappingDriver` reads the mapping metadata located near the
//! code.
//!
//! The PHP trait is the [`ColocatedMappingDriver`] struct, which a driver
//! holds: its methods are the trait's, and
//! [`ColocatedMappingDriver::get_all_class_names`] takes the driver's
//! `isTransient()`.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/ColocatedMappingDriver.php

use std::sync::{Arc, Mutex, PoisonError, RwLock};

use php_value::Thrown;

use crate::mapping::driver::{ClassLocator, FileClassLocator};
use crate::mapping::mapping_exception;

#[derive(Default)]
struct Settings {
    class_locator: Option<Arc<dyn ClassLocator>>,
    /// The directory paths where to look for mapping files.
    paths: Vec<String>,
    /// The paths excluded from path where to look for mapping files.
    exclude_paths: Vec<String>,
    /// The file extension of mapping documents.
    file_extension: String,
}

/// `Doctrine\Persistence\Mapping\Driver\ColocatedMappingDriver`.
pub struct ColocatedMappingDriver {
    /// `static::class` of the driver using the trait.
    driver_class: String,
    settings: RwLock<Settings>,
    /// Cache for {@see `getAllClassNames()`}.
    class_names: Mutex<Option<Vec<String>>>,
}

fn append_unique(list: &mut Vec<String>, paths: &[String]) {
    for path in paths {
        if !list.contains(path) {
            list.push(path.clone());
        }
    }
}

impl ColocatedMappingDriver {
    /// The trait's state for the driver class `driver_class`.
    pub fn new(driver_class: impl Into<String>) -> Self {
        Self {
            driver_class: driver_class.into(),
            settings: RwLock::new(Settings {
                file_extension: ".php".to_owned(),
                ..Settings::default()
            }),
            class_names: Mutex::new(None),
        }
    }

    /// `$this->classLocator = $locator`.
    pub fn set_class_locator(&self, class_locator: Arc<dyn ClassLocator>) {
        self.settings
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .class_locator = Some(class_locator);
    }

    /// Appends lookup paths to metadata driver.
    pub fn add_paths(&self, paths: &[String]) {
        append_unique(
            &mut self
                .settings
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .paths,
            paths,
        );
    }

    /// Retrieves the defined metadata lookup paths.
    pub fn get_paths(&self) -> Vec<String> {
        self.settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .paths
            .clone()
    }

    /// Append exclude lookup paths to a metadata driver.
    pub fn add_exclude_paths(&self, paths: &[String]) {
        append_unique(
            &mut self
                .settings
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .exclude_paths,
            paths,
        );
    }

    /// Retrieve the defined metadata lookup exclude paths.
    pub fn get_exclude_paths(&self) -> Vec<String> {
        self.settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .exclude_paths
            .clone()
    }

    /// Gets the file extension used to look for mapping files under.
    pub fn get_file_extension(&self) -> String {
        self.settings
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .file_extension
            .clone()
    }

    /// Sets the file extension used to look for mapping files under.
    pub fn set_file_extension(&self, file_extension: &str) {
        file_extension.clone_into(
            &mut self
                .settings
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .file_extension,
        );
    }

    /// Gets the names of all mapped classes known to this driver, the
    /// classes `is_transient` (the driver's `isTransient()`) leaves.
    pub fn get_all_class_names(
        &self,
        is_transient: impl Fn(&str) -> Result<bool, Thrown>,
    ) -> Result<Vec<String>, Thrown> {
        if let Some(class_names) = self
            .class_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        {
            return Ok(class_names);
        }

        let (class_locator, paths, exclude_paths, file_extension) = {
            let settings = self.settings.read().unwrap_or_else(PoisonError::into_inner);
            (
                settings.class_locator.clone(),
                settings.paths.clone(),
                settings.exclude_paths.clone(),
                settings.file_extension.clone(),
            )
        };

        if paths.is_empty() && class_locator.is_none() {
            return Err(mapping_exception::path_required_for_driver(
                &self.driver_class,
            ));
        }

        let mut class_names = match &class_locator {
            Some(locator) => locator.get_class_names()?,
            None => Vec::new(),
        };

        if !paths.is_empty() {
            let mut merged =
                FileClassLocator::create_from_directories(&paths, &exclude_paths, &file_extension)?
                    .get_class_names()?;
            append_unique(&mut merged, &class_names);
            class_names = merged;
        }

        let mut kept = Vec::new();
        for class_name in class_names {
            if !is_transient(&class_name)? {
                kept.push(class_name);
            }
        }

        *self
            .class_names
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(kept.clone());
        Ok(kept)
    }
}
