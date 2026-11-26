//! Contract for classes that are potential listeners of a
//! [`NotifyPropertyChanged`](crate::NotifyPropertyChanged) implementor.
//!
//! Upstream: doctrine/persistence src/PropertyChangedListener.php

use php_value::{Object, Value};

/// `Doctrine\Persistence\PropertyChangedListener`.
pub trait PropertyChangedListener: Send + Sync {
    /// Collect information about a property change.
    ///
    /// `sender` is the object on which the property changed,
    /// `property_name` the name of the property that changed, `old_value`
    /// and `new_value` its old and new values.
    fn property_changed(
        &self,
        sender: &Object,
        property_name: &str,
        old_value: &Value,
        new_value: &Value,
    );
}
