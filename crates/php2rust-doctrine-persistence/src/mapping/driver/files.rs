//! The PHP file functions the locators and drivers use: `is_file()`,
//! `is_dir()`, `realpath()`, `SplFileInfo::getBasename($suffix)` and a
//! `RecursiveIteratorIterator` over a `RecursiveDirectoryIterator` with
//! `LEAVES_ONLY` (files, in the order the file system lists them;
//! symbolic links to directories are not followed). Paths are PHP
//! strings; a file name that is not UTF-8 is read lossily.
//!
//! Upstream: none

use std::fs;
use std::path::{Path, PathBuf};

use php_value::Thrown;

/// `is_file($path)`.
pub fn is_file(path: &str) -> bool {
    !path.is_empty() && Path::new(path).is_file()
}

/// `is_dir($path)`.
pub fn is_dir(path: &str) -> bool {
    !path.is_empty() && Path::new(path).is_dir()
}

/// `realpath($path)`: `None` for `false`.
pub fn realpath(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    Some(display(&strip_verbatim(canonical)))
}

/// Windows' `canonicalize()` gives `\\?\C:\...`; PHP's `realpath()` gives
/// `C:\...`.
fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest.to_owned()),
        _ => path,
    }
}

/// A path as a PHP string.
pub fn display(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// `basename($name, $suffix)` of a file name: the suffix is removed when
/// the name ends with it and is longer than it.
pub fn basename_without<'a>(name: &'a str, suffix: &str) -> &'a str {
    if !suffix.is_empty()
        && name.len() > suffix.len()
        && let Some(stripped) = name.strip_suffix(suffix)
    {
        return stripped;
    }
    name
}

/// A file found under a directory.
#[derive(Debug, Clone)]
pub struct FoundFile {
    /// `getPathname()`: the directory's path, the separator and the name.
    pub pathname: String,
    /// `getPath()`: the directory the file is in.
    pub path: String,
    /// `getBasename()`.
    pub basename: String,
}

/// The files under `dir`, recursively, as `RecursiveIteratorIterator`
/// with `LEAVES_ONLY` lists them.
pub fn walk(dir: &str) -> Result<Vec<FoundFile>, Thrown> {
    let trimmed = dir.trim_end_matches(['/', std::path::MAIN_SEPARATOR]);
    let root = if trimmed.is_empty() { dir } else { trimmed };
    let mut found = Vec::new();
    walk_into(Path::new(root), &mut found)?;
    Ok(found)
}

fn walk_into(dir: &Path, found: &mut Vec<FoundFile>) -> Result<(), Thrown> {
    let entries = fs::read_dir(dir).map_err(|error| {
        Thrown::new(
            "UnexpectedValueException",
            format!(
                "RecursiveDirectoryIterator::__construct({}): Failed to open directory: {error}",
                display(dir)
            ),
            0,
        )
    })?;
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let path = entry.path();
        let is_real_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_real_dir {
            walk_into(&path, found)?;
            continue;
        }
        found.push(FoundFile {
            pathname: display(&path),
            path: display(dir),
            basename: entry.file_name().to_string_lossy().into_owned(),
        });
    }
    Ok(())
}
