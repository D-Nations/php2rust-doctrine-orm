//! `ClassLocator` is an interface for classes that can provide a list of
//! class names.
//!
//! Upstream: doctrine/persistence src/Mapping/Driver/ClassLocator.php

use php_value::Thrown;

/// `Doctrine\Persistence\Mapping\Driver\ClassLocator`.
pub trait ClassLocator: Send + Sync {
    /// The class names.
    fn get_class_names(&self) -> Result<Vec<String>, Thrown>;
}
