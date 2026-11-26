//! Resolves a proxy class name to the name of the class it proxies.
//!
//! Upstream: doctrine/persistence src/Mapping/ProxyClassNameResolver.php

/// `Doctrine\Persistence\Mapping\ProxyClassNameResolver`.
pub trait ProxyClassNameResolver: Send + Sync {
    /// The real class name of `class_name` (a proxy class name or the name
    /// of the class itself).
    fn resolve_class_name(&self, class_name: &str) -> String;
}
