//! Contract for a Doctrine persistence layer `ObjectRepository` class to
//! implement.
//!
//! Upstream: doctrine/persistence src/ObjectRepository.php

use php_value::{Object, PhpArray, Thrown, Value};

/// `Doctrine\Persistence\ObjectRepository`.
pub trait ObjectRepository: Send + Sync {
    /// Finds an object by its primary key / identifier.
    fn find(&self, id: &Value) -> Result<Option<Object>, Thrown>;

    /// Finds all objects in the repository.
    fn find_all(&self) -> Result<Vec<Object>, Thrown>;

    /// Finds objects by a set of criteria (field name → value).
    ///
    /// Optionally sorting (field name → `'asc'`, `'desc'`, `'ASC'` or
    /// `'DESC'`) and limiting details can be passed. An implementation may
    /// throw an `UnexpectedValueException` if certain values of the sorting
    /// or limiting details are not supported.
    fn find_by(
        &self,
        criteria: &PhpArray,
        order_by: Option<&PhpArray>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<Object>, Thrown>;

    /// Finds a single object by a set of criteria.
    fn find_one_by(&self, criteria: &PhpArray) -> Result<Option<Object>, Thrown>;

    /// Returns the class name of the object managed by the repository.
    fn get_class_name(&self) -> String;
}
