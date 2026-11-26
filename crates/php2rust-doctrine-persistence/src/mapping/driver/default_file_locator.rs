//! Locates the file that contains the metadata information for a given
//! class name.
//!
//! This behavior is independent of the actual content of the file. It just
//! detects the file which is responsible for the given class name.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/DefaultFileLocator.php

use std::path::MAIN_SEPARATOR_STR as DIRECTORY_SEPARATOR;

use php_value::Thrown;

use crate::mapping::driver::FileLocator;
use crate::mapping::driver::files::{self, basename_without};
use crate::mapping::mapping_exception;

/// `Doctrine\Persistence\Mapping\Driver\DefaultFileLocator`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DefaultFileLocator {
    /// The paths where to look for mapping files.
    paths: Vec<String>,
    /// The file extension of mapping documents.
    file_extension: Option<String>,
}

impl DefaultFileLocator {
    /// Initializes a new `FileDriver` that looks in the given path(s) for
    /// mapping documents and operates in the specified operating mode.
    pub fn new<S: Into<String>>(
        paths: impl IntoIterator<Item = S>,
        file_extension: Option<&str>,
    ) -> Self {
        let mut locator = Self {
            paths: Vec::new(),
            file_extension: file_extension.map(str::to_owned),
        };
        locator.add_paths(paths);
        locator
    }

    /// Appends lookup paths to metadata driver.
    pub fn add_paths<S: Into<String>>(&mut self, paths: impl IntoIterator<Item = S>) {
        for path in paths {
            let path = path.into();
            if !self.paths.contains(&path) {
                self.paths.push(path);
            }
        }
    }

    /// Sets the file extension used to look for mapping files under.
    pub fn set_file_extension(&mut self, file_extension: Option<&str>) {
        self.file_extension = file_extension.map(str::to_owned);
    }

    fn extension(&self) -> &str {
        self.file_extension.as_deref().unwrap_or_default()
    }
}

impl FileLocator for DefaultFileLocator {
    fn find_mapping_file(&self, class_name: &str) -> Result<String, Thrown> {
        let file_name = format!("{}{}", class_name.replace('\\', "."), self.extension());

        // Check whether file exists
        for path in &self.paths {
            let candidate = format!("{path}{DIRECTORY_SEPARATOR}{file_name}");
            if files::is_file(&candidate) {
                return Ok(candidate);
            }
        }

        Err(mapping_exception::mapping_file_not_found(
            class_name, &file_name,
        ))
    }

    fn get_all_class_names(&self, global_basename: &str) -> Result<Vec<String>, Thrown> {
        if self.paths.is_empty() {
            return Ok(Vec::new());
        }

        let mut classes = Vec::new();

        for path in &self.paths {
            if !files::is_dir(path) {
                return Err(
                    mapping_exception::file_mapping_drivers_require_configured_directory_path(
                        Some(path),
                    ),
                );
            }

            for file in files::walk(path)? {
                let file_name = basename_without(&file.basename, self.extension());

                if file_name == file.basename || file_name == global_basename {
                    continue;
                }

                // NOTE: All files found here means classes are not transient!

                classes.push(file_name.replace('.', "\\"));
            }
        }

        Ok(classes)
    }

    fn file_exists(&self, class_name: &str) -> bool {
        let file_name = format!("{}{}", class_name.replace('\\', "."), self.extension());

        // Check whether file exists
        self.paths
            .iter()
            .any(|path| files::is_file(&format!("{path}{DIRECTORY_SEPARATOR}{file_name}")))
    }

    fn get_paths(&self) -> Vec<String> {
        self.paths.clone()
    }

    fn get_file_extension(&self) -> Option<String> {
        self.file_extension.clone()
    }
}
