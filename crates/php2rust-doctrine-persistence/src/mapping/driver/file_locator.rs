//! Locates the file that contains the metadata information for a given
//! class name.
//!
//! This behavior is independent of the actual content of the file. It just
//! detects the file which is responsible for the given class name.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/FileLocator.php

use php_value::Thrown;

/// `Doctrine\Persistence\Mapping\Driver\FileLocator`.
pub trait FileLocator: Send + Sync {
    /// Locates mapping file for the given class name.
    fn find_mapping_file(&self, class_name: &str) -> Result<String, Thrown>;

    /// Gets all class names that are found with this file locator.
    ///
    /// `global_basename` is passed to allow excluding the basename.
    fn get_all_class_names(&self, global_basename: &str) -> Result<Vec<String>, Thrown>;

    /// Checks if a file can be found for this class name.
    fn file_exists(&self, class_name: &str) -> bool;

    /// Gets all the paths that this file locator looks for mapping files.
    fn get_paths(&self) -> Vec<String>;

    /// Gets the file extension that mapping files are suffixed with.
    fn get_file_extension(&self) -> Option<String>;
}
