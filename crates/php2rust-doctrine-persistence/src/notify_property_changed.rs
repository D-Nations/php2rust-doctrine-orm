//! Interface for classes that notify event listeners of changes to their
//! managed properties.
//!
//! This interface is implemented by objects that manually want to notify
//! their object manager or other listeners when properties change, instead
//! of relying on the object manager to compute property changes itself when
//! changes are to be persisted.
//!
//! Upstream: doctrine/persistence src/NotifyPropertyChanged.php

use std::sync::Arc;

use php_value::PhpObject;

use crate::PropertyChangedListener;

/// `Doctrine\Persistence\NotifyPropertyChanged`.
pub trait NotifyPropertyChanged: PhpObject {
    /// Adds a listener that wants to be notified about property changes.
    fn add_property_changed_listener(&self, listener: Arc<dyn PropertyChangedListener>);
}
