//! Base driver for file-based metadata drivers.
//!
//! A file driver operates in a mode where it loads the mapping files of
//! individual classes on demand. This requires the user to adhere to the
//! convention of 1 mapping file per class and the file names of the mapping
//! files must correspond to the full class name, including namespace, with
//! the namespace delimiters '\', replaced by dots '.'.
//!
//! The abstract class is [`FileDriver`] over a [`FileDriverImpl`], which
//! has the abstract methods (`loadMappingFile()` and
//! `loadMetadataForClass()`, which gets the driver to call
//! [`FileDriver::get_element`]).
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/FileDriver.php

use std::sync::{Arc, Mutex, PoisonError, RwLock};

use php_value::Thrown;

use crate::mapping::driver::files;
use crate::mapping::driver::{DefaultFileLocator, FileLocator, MappingDriver};
use crate::mapping::{ClassMetadata, mapping_exception};

/// The abstract methods of `FileDriver`.
pub trait FileDriverImpl: Send + Sync + Sized + 'static {
    /// What a mapping file holds for a class.
    type Element: Clone + Send + Sync;

    /// Loads a mapping file with the given name and returns a map from
    /// class/entity names to their corresponding file driver elements.
    fn load_mapping_file(&self, file: &str) -> Result<Vec<(String, Self::Element)>, Thrown>;

    /// `loadMetadataForClass()` of the concrete driver.
    fn load_metadata_for_class(
        &self,
        driver: &FileDriver<Self>,
        class_name: &str,
        metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown>;
}

/// Where the driver's mapping files are: a locator, or paths for a
/// [`DefaultFileLocator`].
pub enum FileDriverLocator {
    Locator(Arc<dyn FileLocator>),
    Paths(Vec<String>),
}

impl From<Arc<dyn FileLocator>> for FileDriverLocator {
    fn from(locator: Arc<dyn FileLocator>) -> Self {
        Self::Locator(locator)
    }
}

impl From<&str> for FileDriverLocator {
    fn from(path: &str) -> Self {
        Self::Paths(vec![path.to_owned()])
    }
}

impl From<Vec<String>> for FileDriverLocator {
    fn from(paths: Vec<String>) -> Self {
        Self::Paths(paths)
    }
}

/// Class name → element, in the order the classes were loaded.
type ClassCache<E> = Vec<(String, E)>;

/// `Doctrine\Persistence\Mapping\Driver\FileDriver`.
pub struct FileDriver<T: FileDriverImpl> {
    inner: T,
    locator: RwLock<Arc<dyn FileLocator>>,
    class_cache: Mutex<Option<ClassCache<T::Element>>>,
    global_basename: RwLock<String>,
}

fn upsert<E>(cache: &mut Vec<(String, E)>, name: String, element: E) {
    match cache.iter_mut().find(|(n, _)| *n == name) {
        Some(entry) => entry.1 = element,
        None => cache.push((name, element)),
    }
}

impl<T: FileDriverImpl> FileDriver<T> {
    /// Initializes a new `FileDriver` that looks in the given path(s) for
    /// mapping documents and operates in the specified operating mode.
    pub fn new(
        inner: T,
        locator: impl Into<FileDriverLocator>,
        file_extension: Option<&str>,
    ) -> Self {
        let locator: Arc<dyn FileLocator> = match locator.into() {
            FileDriverLocator::Locator(locator) => locator,
            FileDriverLocator::Paths(paths) => {
                Arc::new(DefaultFileLocator::new(paths, file_extension))
            }
        };
        Self {
            inner,
            locator: RwLock::new(locator),
            class_cache: Mutex::new(None),
            global_basename: RwLock::new(String::new()),
        }
    }

    /// The concrete driver.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Sets the global basename.
    pub fn set_global_basename(&self, file: &str) {
        file.clone_into(
            &mut self
                .global_basename
                .write()
                .unwrap_or_else(PoisonError::into_inner),
        );
    }

    /// Retrieves the global basename.
    pub fn get_global_basename(&self) -> String {
        self.global_basename
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Gets the element of schema meta data for the class from the mapping
    /// file. This will lazily load the mapping file if it is not loaded
    /// yet.
    pub fn get_element(&self, class_name: &str) -> Result<T::Element, Thrown> {
        self.ensure_initialized()?;

        if let Some(element) = self.cached(class_name) {
            return Ok(element);
        }

        let locator = self.get_locator();
        let result = self
            .inner
            .load_mapping_file(&locator.find_mapping_file(class_name)?)?;

        let Some((_, element)) = result.into_iter().find(|(name, _)| name == class_name) else {
            return Err(mapping_exception::invalid_mapping_file(
                class_name,
                &format!(
                    "{}{}",
                    class_name.replace('\\', "."),
                    locator.get_file_extension().unwrap_or_default()
                ),
            ));
        };

        let mut cache = self
            .class_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        upsert(
            cache.get_or_insert_with(Vec::new),
            class_name.to_owned(),
            element.clone(),
        );

        Ok(element)
    }

    fn cached(&self, class_name: &str) -> Option<T::Element> {
        self.class_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()?
            .iter()
            .find(|(name, _)| name == class_name)
            .map(|(_, element)| element.clone())
    }

    fn ensure_initialized(&self) -> Result<(), Thrown> {
        let initialized = self
            .class_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some();
        if !initialized {
            self.initialize()?;
        }
        Ok(())
    }

    /// Initializes the class cache from all the global files.
    ///
    /// Using this feature adds a substantial performance hit to file
    /// drivers as more metadata has to be loaded into memory than might
    /// actually be necessary. This may not be relevant to scenarios where
    /// caching of metadata is in place, however hits very hard in
    /// scenarios where no caching is used.
    pub fn initialize(&self) -> Result<(), Thrown> {
        let mut class_cache = Vec::new();
        let global_basename = self.get_global_basename();
        if !global_basename.is_empty() {
            let locator = self.get_locator();
            for path in locator.get_paths() {
                let file = format!(
                    "{path}/{global_basename}{}",
                    locator.get_file_extension().unwrap_or_default()
                );
                if !files::is_file(&file) {
                    continue;
                }

                for (name, element) in self.inner.load_mapping_file(&file)? {
                    upsert(&mut class_cache, name, element);
                }
            }
        }
        *self
            .class_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(class_cache);
        Ok(())
    }

    /// Retrieves the locator used to discover mapping files by className.
    pub fn get_locator(&self) -> Arc<dyn FileLocator> {
        Arc::clone(&self.locator.read().unwrap_or_else(PoisonError::into_inner))
    }

    /// Sets the locator used to discover mapping files by className.
    pub fn set_locator(&self, locator: Arc<dyn FileLocator>) {
        *self.locator.write().unwrap_or_else(PoisonError::into_inner) = locator;
    }
}

impl<T: FileDriverImpl> MappingDriver for FileDriver<T> {
    fn load_metadata_for_class(
        &self,
        class_name: &str,
        metadata: &dyn ClassMetadata,
    ) -> Result<(), Thrown> {
        self.inner
            .load_metadata_for_class(self, class_name, metadata)
    }

    fn is_transient(&self, class_name: &str) -> Result<bool, Thrown> {
        self.ensure_initialized()?;

        if self.cached(class_name).is_some() {
            return Ok(false);
        }

        Ok(!self.get_locator().file_exists(class_name))
    }

    fn get_all_class_names(&self) -> Result<Vec<String>, Thrown> {
        self.ensure_initialized()?;

        let keys: Vec<String> = self
            .class_cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|cache| cache.iter().map(|(name, _)| name.clone()).collect())
            .unwrap_or_default();

        let located = self
            .get_locator()
            .get_all_class_names(&self.get_global_basename())?;
        if keys.is_empty() {
            return Ok(located);
        }

        let mut class_names = keys;
        for name in located {
            if !class_names.contains(&name) {
                class_names.push(name);
            }
        }
        Ok(class_names)
    }
}
