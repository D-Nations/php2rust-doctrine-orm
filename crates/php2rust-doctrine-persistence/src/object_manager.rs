//! Contract for a Doctrine persistence layer `ObjectManager` class to
//! implement.
//!
//! Upstream: doctrine/persistence src/ObjectManager.php

use std::sync::Arc;

use php_value::{Object, Thrown, Value};

use crate::ObjectRepository;
use crate::mapping::{ClassMetadata, ClassMetadataFactory};

/// `Doctrine\Persistence\ObjectManager`.
pub trait ObjectManager: Send + Sync {
    /// Finds an object by its identifier.
    ///
    /// This is just a convenient shortcut for
    /// `getRepository($className)->find($id)`.
    fn find(&self, class_name: &str, id: &Value) -> Result<Option<Object>, Thrown>;

    /// Tells the `ObjectManager` to make an instance managed and persistent.
    ///
    /// The object will be entered into the database as a result of the
    /// flush operation.
    ///
    /// NOTE: The persist operation always considers objects that are not
    /// yet known to this `ObjectManager` as NEW. Do not pass detached objects
    /// to the persist operation.
    fn persist(&self, object: &Object) -> Result<(), Thrown>;

    /// Removes an object instance.
    ///
    /// A removed object will be removed from the database as a result of
    /// the flush operation.
    fn remove(&self, object: &Object) -> Result<(), Thrown>;

    /// Clears the `ObjectManager`. All objects that are currently managed by
    /// this `ObjectManager` become detached.
    fn clear(&self) -> Result<(), Thrown>;

    /// Detaches an object from the `ObjectManager`, causing a managed object
    /// to become detached. Unflushed changes made to the object if any
    /// (including removal of the object), will not be synchronized to the
    /// database. Objects which previously referenced the detached object
    /// will continue to reference it.
    fn detach(&self, object: &Object) -> Result<(), Thrown>;

    /// Refreshes the persistent state of an object from the database,
    /// overriding any local changes that have not yet been persisted.
    fn refresh(&self, object: &Object) -> Result<(), Thrown>;

    /// Flushes all changes to objects that have been queued up to now to
    /// the database. This effectively synchronizes the in-memory state of
    /// managed objects with the database.
    fn flush(&self) -> Result<(), Thrown>;

    /// Gets the repository for a class.
    fn get_repository(&self, class_name: &str) -> Result<Arc<dyn ObjectRepository>, Thrown>;

    /// Returns the `ClassMetadata` descriptor for a class.
    ///
    /// The class name must be the fully-qualified class name without a
    /// leading backslash (as it is returned by `get_class($obj)`).
    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<dyn ClassMetadata>, Thrown>;

    /// Gets the metadata factory used to gather the metadata of classes.
    fn get_metadata_factory(&self) -> Arc<dyn ClassMetadataFactory>;

    /// Helper method to initialize a lazy loading proxy or persistent
    /// collection.
    ///
    /// This method is a no-op for other objects.
    fn initialize_object(&self, obj: &Object) -> Result<(), Thrown>;

    /// Helper method to check whether a lazy loading proxy or persistent
    /// collection has been initialized.
    fn is_uninitialized_object(&self, value: &Value) -> bool;

    /// Checks if the object is part of the current `UnitOfWork` and therefore
    /// managed.
    fn contains(&self, object: &Object) -> bool;
}
