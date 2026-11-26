//! `ClassLocator` implementation that uses a list of file names to locate
//! PHP files and extract class names from them.
//!
//! It is compatible with the Symfony Finder component, but does not
//! require it.
//!
//! PHP `require_once`s each file and keeps the declared classes whose
//! `ReflectionClass::getFileName()` is one of them. A Rust program
//! declares its classes up front (`php_reflection::register_class`, with
//! the file through `ClassInfo::file`), so including a file is a no-op
//! and the classes are the registered ones declared in the given files,
//! in declaration order.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/FileClassLocator.php

use std::collections::HashSet;
use std::path::PathBuf;

use php_value::Thrown;

use crate::mapping::driver::ClassLocator;
use crate::mapping::driver::files::{self, display};
use crate::mapping::mapping_exception;
use crate::php_reflection;

/// `Doctrine\Persistence\Mapping\Driver\FileClassLocator`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileClassLocator {
    /// An iterable of files to include (`SplFileInfo`s).
    files: Vec<PathBuf>,
}

impl FileClassLocator {
    pub fn new<P: Into<PathBuf>>(files: impl IntoIterator<Item = P>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
        }
    }

    /// Creates a [`FileClassLocator`] from an array of directories.
    ///
    /// `directories` are the directories to search for PHP files,
    /// `excluded_directories` the directories to exclude from the search,
    /// `file_extension` the file extension to look for (default is
    /// `.php`).
    pub fn create_from_directories(
        directories: &[String],
        excluded_directories: &[String],
        file_extension: &str,
    ) -> Result<Self, Thrown> {
        let mut found = Vec::new();
        for directory in directories {
            if !files::is_dir(directory) {
                return Err(
                    mapping_exception::file_mapping_drivers_require_configured_directory_path(
                        Some(directory),
                    ),
                );
            }

            found.extend(
                files::walk(directory)?
                    .into_iter()
                    .filter(|file| file.pathname.ends_with(file_extension))
                    .map(|file| file.pathname),
            );
        }

        if !excluded_directories.is_empty() {
            let excluded: Vec<String> = excluded_directories
                .iter()
                // realpath() returns false if the file is in a phar archive
                .map(|dir| {
                    files::realpath(dir)
                        .unwrap_or_else(|| dir.clone())
                        .replace('\\', "/")
                })
                .collect();
            found.retain(|file| {
                let source_file = files::realpath(file)
                    .unwrap_or_else(|| file.clone())
                    .replace('\\', "/");
                !excluded
                    .iter()
                    .any(|excluded_directory| source_file.starts_with(excluded_directory.as_str()))
            });
        }

        Ok(Self::new(found))
    }
}

impl ClassLocator for FileClassLocator {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        let mut included_files: HashSet<String> = HashSet::new();

        for file in &self.files {
            // Skip non-files
            if !file.is_file() {
                continue;
            }

            // getRealPath() returns false if the file is in a phar archive
            let pathname = display(file);
            let file_name = files::realpath(&pathname).unwrap_or(pathname);
            included_files.insert(file_name);
        }

        let mut classes = Vec::new();
        for class in php_reflection::declared() {
            let Some(file_name) = class.file_name() else {
                continue;
            };
            let file_name = display(file_name);
            let file_name = files::realpath(&file_name).unwrap_or(file_name);
            if !included_files.contains(&file_name) {
                continue;
            }
            if class.kind() == php_reflection::ClassKind::Class
                || class.kind() == php_reflection::ClassKind::Enum
            {
                classes.push(class.name().to_owned());
            }
        }

        Ok(classes)
    }
}
