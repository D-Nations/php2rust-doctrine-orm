//! Base class to simplify `ObjectManager` decorators.
//!
//! The abstract class is a struct over the wrapped manager that delegates
//! every method; a decorator wraps it (or implements [`ObjectManager`]
//! itself, calling [`ObjectManagerDecorator::wrapped`] for what it does
//! not change).
//!
//! Upstream: doctrine/persistence src/ObjectManagerDecorator.php

use std::sync::Arc;

use php_value::{Object, Thrown, Value};

use crate::mapping::{ClassMetadata, ClassMetadataFactory};
use crate::{ObjectManager, ObjectRepository};

/// `Doctrine\Persistence\ObjectManagerDecorator`.
pub struct ObjectManagerDecorator<T: ObjectManager + ?Sized> {
    wrapped: Arc<T>,
}

impl<T: ObjectManager + ?Sized> ObjectManagerDecorator<T> {
    pub fn new(wrapped: Arc<T>) -> Self {
        Self { wrapped }
    }

    /// `$this->wrapped`.
    pub fn wrapped(&self) -> &Arc<T> {
        &self.wrapped
    }
}

impl<T: ObjectManager + ?Sized> ObjectManager for ObjectManagerDecorator<T> {
    fn find(&self, class_name: &str, id: &Value) -> Result<Option<Object>, Thrown> {
        self.wrapped.find(class_name, id)
    }

    fn persist(&self, object: &Object) -> Result<(), Thrown> {
        self.wrapped.persist(object)
    }

    fn remove(&self, object: &Object) -> Result<(), Thrown> {
        self.wrapped.remove(object)
    }

    fn clear(&self) -> Result<(), Thrown> {
        self.wrapped.clear()
    }

    fn detach(&self, object: &Object) -> Result<(), Thrown> {
        self.wrapped.detach(object)
    }

    fn refresh(&self, object: &Object) -> Result<(), Thrown> {
        self.wrapped.refresh(object)
    }

    fn flush(&self) -> Result<(), Thrown> {
        self.wrapped.flush()
    }

    fn get_repository(&self, class_name: &str) -> Result<Arc<dyn ObjectRepository>, Thrown> {
        self.wrapped.get_repository(class_name)
    }

    fn get_class_metadata(&self, class_name: &str) -> Result<Arc<dyn ClassMetadata>, Thrown> {
        self.wrapped.get_class_metadata(class_name)
    }

    fn get_metadata_factory(&self) -> Arc<dyn ClassMetadataFactory> {
        self.wrapped.get_metadata_factory()
    }

    fn initialize_object(&self, obj: &Object) -> Result<(), Thrown> {
        self.wrapped.initialize_object(obj)
    }

    fn is_uninitialized_object(&self, value: &Value) -> bool {
        self.wrapped.is_uninitialized_object(value)
    }

    fn contains(&self, object: &Object) -> bool {
        self.wrapped.contains(object)
    }
}
