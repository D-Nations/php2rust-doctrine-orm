//! `SymfonyFileLocator` uses Symfony 2 / 3 / 4 style mapping files, for
//! some given namespace prefix (paths map to namespace prefixes).
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/SymfonyFileLocator.php

use std::path::MAIN_SEPARATOR_STR as DIRECTORY_SEPARATOR;

use php_value::Thrown;

use crate::mapping::driver::FileLocator;
use crate::mapping::driver::files::{self, basename_without};
use crate::mapping::mapping_exception;

/// `Doctrine\Persistence\Mapping\Driver\SymfonyFileLocator`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymfonyFileLocator {
    /// The paths where to look for mapping files.
    paths: Vec<String>,
    /// A map of mapping directory path to namespace prefix used to expand
    /// class shortnames.
    prefixes: Vec<(String, String)>,
    /// File extension that is searched for.
    file_extension: Option<String>,
    /// Represents PHP namespace delimiters when looking for files.
    ns_separator: String,
}

impl SymfonyFileLocator {
    /// `prefixes` maps paths to namespace prefixes; `ns_separator` is `.`
    /// by default (the `InvalidArgumentException` for an empty one).
    pub fn new(
        prefixes: Vec<(String, String)>,
        file_extension: Option<&str>,
        ns_separator: &str,
    ) -> Result<Self, Thrown> {
        let mut locator = Self {
            paths: Vec::new(),
            prefixes: Vec::new(),
            file_extension: file_extension.map(str::to_owned),
            ns_separator: ns_separator.to_owned(),
        };
        locator.add_namespace_prefixes(prefixes);

        if ns_separator.is_empty() {
            return Err(Thrown::new(
                "InvalidArgumentException",
                "Namespace separator should not be empty",
                0,
            ));
        }

        Ok(locator)
    }

    /// Adds Namespace Prefixes.
    pub fn add_namespace_prefixes(&mut self, prefixes: Vec<(String, String)>) {
        for (path, prefix) in prefixes {
            match self.prefixes.iter_mut().find(|(p, _)| *p == path) {
                Some(entry) => entry.1 = prefix,
                None => self.prefixes.push((path.clone(), prefix)),
            }
            self.paths.push(path);
        }
    }

    /// Gets Namespace Prefixes.
    pub fn get_namespace_prefixes(&self) -> Vec<(String, String)> {
        self.prefixes.clone()
    }

    pub fn set_file_extension(&mut self, file_extension: &str) {
        self.file_extension = Some(file_extension.to_owned());
    }

    fn extension(&self) -> &str {
        self.file_extension.as_deref().unwrap_or_default()
    }

    fn prefix(&self, path: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .find(|(p, _)| p == path)
            .map(|(_, prefix)| prefix.as_str())
    }

    /// The file for `class_name` in each path, as `fileExists()` and
    /// `findMappingFile()` look for it.
    fn candidates(&self, class_name: &str) -> Vec<String> {
        let default_file_name = format!(
            "{}{}",
            class_name.replace('\\', &self.ns_separator),
            self.extension()
        );
        let mut candidates = Vec::new();
        for path in &self.paths {
            let Some(prefix) = self.prefix(path) else {
                // global namespace class
                candidates.push(format!("{path}{DIRECTORY_SEPARATOR}{default_file_name}"));
                continue;
            };

            let Some(rest) = class_name.strip_prefix(&format!("{prefix}\\")) else {
                continue;
            };

            candidates.push(format!(
                "{path}/{}{}",
                rest.replace(
                    '\\',
                    &self.ns_separator[..first_char_len(&self.ns_separator)]
                ),
                self.extension()
            ));
        }
        candidates
    }

    fn realpath(path: &str) -> Result<String, Thrown> {
        files::realpath(path).ok_or_else(|| {
            Thrown::new(
                "RuntimeException",
                format!("Could not get realpath for {path}"),
                0,
            )
        })
    }
}

/// `strtr($s, '\\', $sep)` replaces `\` by the first byte of `$sep` only.
fn first_char_len(separator: &str) -> usize {
    separator.chars().next().map_or(0, char::len_utf8)
}

/// `strtr($subject, $from, $to)` with one-character strings.
fn strtr_char(subject: &str, from: &str, to: &str) -> String {
    let (Some(from), Some(to)) = (from.chars().next(), to.chars().next()) else {
        return subject.to_owned();
    };
    subject
        .chars()
        .map(|c| if c == from { to } else { c })
        .collect()
}

impl FileLocator for SymfonyFileLocator {
    fn file_exists(&self, class_name: &str) -> bool {
        self.candidates(class_name)
            .iter()
            .any(|f| files::is_file(f))
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
                let class = if let Some(prefix) = self.prefix(path) {
                    // Calculate namespace suffix for given prefix as a relative path from basepath to file path
                    let file_path = Self::realpath(&file.path)?;
                    let base_path = Self::realpath(path)?;
                    let ns_suffix = strtr_char(
                        file_path.get(base_path.len()..).unwrap_or_default(),
                        &self.ns_separator,
                        "\\",
                    );

                    format!(
                        "{prefix}{}\\{}",
                        ns_suffix.replace(DIRECTORY_SEPARATOR, "\\"),
                        file_name.replace(&self.ns_separator, "\\")
                    )
                } else {
                    file_name.replace(&self.ns_separator, "\\")
                };

                classes.push(class);
            }
        }

        Ok(classes)
    }

    fn find_mapping_file(&self, class_name: &str) -> Result<String, Thrown> {
        if let Some(found) = self
            .candidates(class_name)
            .into_iter()
            .find(|f| files::is_file(f))
        {
            return Ok(found);
        }

        let short_name = class_name.rsplit(['\\', '/']).next().unwrap_or(class_name);
        Err(mapping_exception::mapping_file_not_found(
            class_name,
            &format!("{short_name}{}", self.extension()),
        ))
    }

    fn get_paths(&self) -> Vec<String> {
        self.paths.clone()
    }

    fn get_file_extension(&self) -> Option<String> {
        self.file_extension.clone()
    }
}
