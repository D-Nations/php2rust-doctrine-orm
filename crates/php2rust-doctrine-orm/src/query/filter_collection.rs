//! Collection class for all the query filters.
//!
//! The entity manager holds its collection (PHP creates it on the first
//! `getFilters()`); the collection holds the entity manager weakly.
//!
//! Upstream: doctrine/orm src/Query/FilterCollection.php

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use php_value::Thrown;

use crate::entity_manager::EntityManager;
use crate::query::filter::{SqlFilter, new_filter};

/// A filter object is in CLEAN state when it has no changed parameters.
pub const FILTERS_STATE_CLEAN: i64 = 1;

/// A filter object is in DIRTY state when it has changed parameters.
pub const FILTERS_STATE_DIRTY: i64 = 2;

#[derive(Debug)]
struct CollectionState {
    /// Instances of enabled filters, sorted by name.
    enabled_filters: BTreeMap<String, Arc<SqlFilter>>,
    /// The filter hash from the last time the query was parsed.
    filter_hash: String,
    /// Instances of suspended filters, in the order they were suspended.
    suspended_filters: Vec<(String, Arc<SqlFilter>)>,
    /// The current state of this filter.
    filters_state: i64,
}

/// `Doctrine\ORM\Query\FilterCollection`.
#[derive(Debug)]
pub struct FilterCollection {
    em: Weak<EntityManager>,
    state: Mutex<CollectionState>,
}

fn invalid_argument(message: String) -> Thrown {
    Thrown::new("InvalidArgumentException", message, 0)
}

impl FilterCollection {
    pub const CLASS: &'static str = "Doctrine\\ORM\\Query\\FilterCollection";

    pub fn new(em: Weak<EntityManager>) -> Self {
        Self {
            em,
            state: Mutex::new(CollectionState {
                enabled_filters: BTreeMap::new(),
                filter_hash: String::new(),
                suspended_filters: Vec::new(),
                filters_state: FILTERS_STATE_CLEAN,
            }),
        }
    }

    fn st(&self) -> MutexGuard<'_, CollectionState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em.upgrade().ok_or_else(|| {
            Thrown::new(
                "Error",
                "The entity manager of the filter collection is gone",
                0,
            )
        })
    }

    /// Gets all the enabled filters.
    pub fn get_enabled_filters(&self) -> Vec<(String, Arc<SqlFilter>)> {
        self.st()
            .enabled_filters
            .iter()
            .map(|(name, filter)| (name.clone(), filter.clone()))
            .collect()
    }

    /// Gets all the suspended filters.
    pub fn get_suspended_filters(&self) -> Vec<(String, Arc<SqlFilter>)> {
        self.st().suspended_filters.clone()
    }

    /// Enables a filter from the collection.
    ///
    /// Throws an `InvalidArgumentException` if the filter does not exist.
    pub fn enable(&self, name: &str) -> Result<Arc<SqlFilter>, Thrown> {
        if !self.has(name)? {
            return Err(invalid_argument(format!("Filter '{name}' does not exist.")));
        }

        if !self.is_enabled(name) {
            let em = self.em()?;
            let Some(filter_class) = em.get_configuration().get_filter_class_name(name) else {
                return Err(Thrown::new(
                    "AssertionError",
                    "assert($filterClass !== null)",
                    0,
                ));
            };

            let filter = Arc::new(new_filter(&filter_class, &em)?);
            let mut st = self.st();
            st.enabled_filters.insert(name.to_owned(), filter);

            // In case a suspended filter with the same name was forgotten
            st.suspended_filters.retain(|(n, _)| n != name);

            // Keep the enabled filters sorted for the hash (a BTreeMap)

            st.filters_state = FILTERS_STATE_DIRTY;
        }

        self.get_filter(name)
    }

    /// Disables a filter.
    ///
    /// Throws an `InvalidArgumentException` if the filter does not exist.
    pub fn disable(&self, name: &str) -> Result<Arc<SqlFilter>, Thrown> {
        // Get the filter to return it
        let filter = self.get_filter(name)?;

        let mut st = self.st();
        st.enabled_filters.remove(name);

        st.filters_state = FILTERS_STATE_DIRTY;

        Ok(filter)
    }

    /// Suspend a filter: the suspended filter.
    ///
    /// Throws an `InvalidArgumentException` if the filter does not exist.
    pub fn suspend(&self, name: &str) -> Result<Arc<SqlFilter>, Thrown> {
        // Get the filter to return it
        let filter = self.get_filter(name)?;

        let mut st = self.st();
        match st.suspended_filters.iter_mut().find(|(n, _)| n == name) {
            Some(entry) => entry.1 = filter.clone(),
            None => st.suspended_filters.push((name.to_owned(), filter.clone())),
        }
        st.enabled_filters.remove(name);

        st.filters_state = FILTERS_STATE_DIRTY;

        Ok(filter)
    }

    /// Restore a disabled filter from the collection: the restored filter.
    ///
    /// Throws an `InvalidArgumentException` if the filter does not exist.
    pub fn restore(&self, name: &str) -> Result<Arc<SqlFilter>, Thrown> {
        if !self.is_suspended(name) {
            return Err(invalid_argument(format!(
                "Filter '{name}' is not suspended."
            )));
        }

        let mut st = self.st();
        let Some(at) = st.suspended_filters.iter().position(|(n, _)| n == name) else {
            return Err(invalid_argument(format!(
                "Filter '{name}' is not suspended."
            )));
        };
        let (_, filter) = st.suspended_filters.remove(at);
        st.enabled_filters.insert(name.to_owned(), filter.clone());

        // Keep the enabled filters sorted for the hash (a BTreeMap)

        st.filters_state = FILTERS_STATE_DIRTY;

        Ok(filter)
    }

    /// Gets an enabled filter from the collection.
    ///
    /// Throws an `InvalidArgumentException` if the filter is not enabled.
    pub fn get_filter(&self, name: &str) -> Result<Arc<SqlFilter>, Thrown> {
        self.st()
            .enabled_filters
            .get(name)
            .cloned()
            .ok_or_else(|| invalid_argument(format!("Filter '{name}' is not enabled.")))
    }

    /// Checks whether filter with given name is defined.
    pub fn has(&self, name: &str) -> Result<bool, Thrown> {
        Ok(self
            .em()?
            .get_configuration()
            .get_filter_class_name(name)
            .is_some())
    }

    /// Checks if a filter is enabled.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.st().enabled_filters.contains_key(name)
    }

    /// Checks if a filter is suspended.
    pub fn is_suspended(&self, name: &str) -> bool {
        self.st().suspended_filters.iter().any(|(n, _)| n == name)
    }

    /// Checks if the filter collection is clean.
    pub fn is_clean(&self) -> bool {
        self.st().filters_state == FILTERS_STATE_CLEAN
    }

    /// Generates a string of currently enabled filters to use for the cache
    /// id.
    pub fn get_hash(&self) -> String {
        let mut st = self.st();
        // If there are only clean filters, the previous hash can be returned
        if st.filters_state == FILTERS_STATE_CLEAN {
            return st.filter_hash.clone();
        }

        let mut filter_hash = String::new();

        for (name, filter) in &st.enabled_filters {
            filter_hash.push_str(name);
            filter_hash.push_str(&filter.to_php_string().to_string_lossy());
        }

        st.filter_hash.clone_from(&filter_hash);
        st.filters_state = FILTERS_STATE_CLEAN;

        filter_hash
    }

    /// Sets the filter state to dirty.
    pub fn set_filters_state_dirty(&self) {
        self.st().filters_state = FILTERS_STATE_DIRTY;
    }
}
