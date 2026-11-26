//! Interface for proxy classes.
//!
//! Upstream: doctrine/persistence src/Proxy.php

use php_value::PhpObject;

/// `Proxy::MARKER`: marker for Proxy class names.
pub const MARKER: &str = "__CG__";

/// `Proxy::MARKER_LENGTH`: length of the proxy marker.
pub const MARKER_LENGTH: usize = 6;

/// The interface's class name.
pub const CLASS: &str = "Doctrine\\Persistence\\Proxy";

/// `Doctrine\Persistence\Proxy`: interface for proxy classes.
pub trait Proxy: PhpObject {
    /// `__load()`: initializes this proxy if its not yet initialized.
    ///
    /// Acts as a no-op if already initialized.
    fn load(&self) -> Result<(), php_value::Thrown>;

    /// `__isInitialized()`: returns whether this proxy is initialized or
    /// not.
    fn is_initialized(&self) -> bool;

    /// `__setInitialized($initialized)`, which proxies may have (it is not
    /// part of the interface): `None` when the class has no such method.
    fn set_initialized(&self, initialized: bool) -> Option<()> {
        let _ = initialized;
        None
    }
}
