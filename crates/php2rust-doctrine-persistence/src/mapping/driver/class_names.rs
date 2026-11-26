//! Basic implementation of `ClassLocator` that passes a list of class names.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/ClassNames.php

use php_value::Thrown;

use crate::mapping::driver::ClassLocator;

/// `Doctrine\Persistence\Mapping\Driver\ClassNames`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassNames {
    class_names: Vec<String>,
}

impl ClassNames {
    pub fn new<S: Into<String>>(class_names: impl IntoIterator<Item = S>) -> Self {
        Self {
            class_names: class_names.into_iter().map(Into::into).collect(),
        }
    }
}

impl ClassLocator for ClassNames {
    fn get_class_names(&self) -> Result<Vec<String>, Thrown> {
        Ok(self.class_names.clone())
    }
}
