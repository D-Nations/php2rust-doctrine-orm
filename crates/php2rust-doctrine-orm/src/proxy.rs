//! `Doctrine\ORM\Proxy\ProxyFactory`: the port only has native lazy
//! objects (PHP 8.4), so a proxy is a lazy ghost of the entity
//! (`EntityManager::get_proxy()`) and no proxy class is generated.
//! `Autoloader::resolveFile()` is ported; `Autoloader::register()`
//! (`spl_autoload_register()` of a callback that `require`s the proxy
//! file) is not, as no PHP class is loaded.
//!
//! Upstream: doctrine/orm src/Proxy/ProxyFactory.php src/Proxy/Autoloader.php src/Proxy/NotAProxyClass.php

use std::sync::{Arc, Weak};

use php_value::{Object, PhpArray, Thrown};

use crate::configuration::proxy_factory::{AUTOGENERATE_ALWAYS, AUTOGENERATE_NEVER};
use crate::entity_manager::EntityManager;
use crate::mapping::ClassMetadata;

/// `https://github.com/doctrine/orm/pull/12005`.
const PROXY_DEPRECATION: &str = "https://github.com/doctrine/orm/pull/12005";

/// The highest `AUTOGENERATE_*` mode (`AUTOGENERATE_FILE_NOT_EXISTS_OR_CHANGED`).
const AUTOGENERATE_MAX: i64 = 4;

/// This factory is used to create proxy objects for entities at runtime.
#[derive(Debug, Clone)]
pub struct ProxyFactory {
    em: Weak<EntityManager>,
    proxy_dir: String,
    proxy_ns: String,
    auto_generate: i64,
}

/// The checks of `new ProxyFactory()` for a configuration without
/// native lazy objects: the port has no `LazyGhost` proxies, as
/// `ProxyHelper::generateLazyGhost()` is missing with
/// symfony/var-exporter 8 (the version PHP 8.4 installs).
pub(crate) fn check_lazy_objects(
    config: &crate::configuration::Configuration,
) -> Result<(), Thrown> {
    if !config.is_native_lazy_objects_enabled() {
        crate::deprecations::trigger(PROXY_DEPRECATION, || {
            "Not enabling native lazy objects is deprecated and will be impossible in Doctrine ORM 4.0.".to_owned()
        });

        return Err(crate::exception::lazy_ghost_unavailable());
    }
    Ok(())
}

impl ProxyFactory {
    /// `new ProxyFactory($em)`.
    pub fn new(em: &Arc<EntityManager>) -> Self {
        Self {
            em: Arc::downgrade(em),
            proxy_dir: String::new(),
            proxy_ns: String::new(),
            auto_generate: AUTOGENERATE_NEVER,
        }
    }

    /// `new ProxyFactory($em, $proxyDir, $proxyNs, $autoGenerate)`: more
    /// than just the entity manager, which PHP 8.4 deprecates.
    pub fn with_proxy_args(
        em: &Arc<EntityManager>,
        proxy_dir: Option<&str>,
        proxy_ns: Option<&str>,
        auto_generate: i64,
    ) -> Result<Self, Thrown> {
        crate::deprecations::trigger(PROXY_DEPRECATION, || {
            "Passing more than just the EntityManager to the Doctrine\\ORM\\Proxy\\ProxyFactory::__construct is deprecated and will not be possible in Doctrine ORM 4.0.".to_owned()
        });
        if !(AUTOGENERATE_NEVER..=AUTOGENERATE_MAX).contains(&auto_generate) {
            return Err(crate::exception::invalid_auto_generate_mode(auto_generate));
        }
        Ok(Self {
            em: Arc::downgrade(em),
            proxy_dir: proxy_dir.unwrap_or_default().to_owned(),
            proxy_ns: proxy_ns.unwrap_or_default().to_owned(),
            auto_generate,
        })
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em.upgrade().ok_or_else(|| {
            Thrown::new(
                "Error",
                "Call to a member function getClassMetadata() on null",
                0,
            )
        })
    }

    /// The proxy directory (`''` with native lazy objects).
    pub fn proxy_dir(&self) -> &str {
        &self.proxy_dir
    }

    /// The proxy namespace (`''` with native lazy objects).
    pub fn proxy_ns(&self) -> &str {
        &self.proxy_ns
    }

    /// Whether proxy classes are generated in every request.
    pub fn is_auto_generate_always(&self) -> bool {
        self.auto_generate == AUTOGENERATE_ALWAYS
    }

    /// `getProxy($className, $identifier)`: an uninitialized lazy ghost of
    /// the class with its identifier set; initializing it loads the entity
    /// through the class's persister (`loadById()`), or throws
    /// `EntityNotFoundException` and leaves it uninitialized.
    pub fn get_proxy(&self, class_name: &str, identifier: &PhpArray) -> Result<Object, Thrown> {
        self.em()?.get_proxy(class_name, identifier)
    }

    /// Generates proxy classes for all given classes: none with native lazy
    /// objects.
    pub fn generate_proxy_classes(
        &self,
        _classes: &[Arc<ClassMetadata>],
        _proxy_dir: Option<&str>,
    ) -> usize {
        0
    }
}

/// `Doctrine\ORM\Proxy\NotAProxyClass`.
pub const NOT_A_PROXY_CLASS: &str = "Doctrine\\ORM\\Proxy\\NotAProxyClass";

/// `Doctrine\ORM\Proxy\Autoloader`.
#[derive(Debug, Clone, Copy)]
pub struct Autoloader;

impl Autoloader {
    /// Resolves proxy class name to a filename based on the following
    /// pattern.
    ///
    /// 1. Remove Proxy namespace from class name.
    /// 2. Remove namespace separators from remaining class name.
    /// 3. Return PHP filename from proxy-dir with the result from 2.
    pub fn resolve_file(
        proxy_dir: &str,
        proxy_namespace: &str,
        class_name: &str,
    ) -> Result<String, Thrown> {
        crate::deprecations::trigger(PROXY_DEPRECATION, || {
            "Class \"Doctrine\\ORM\\Proxy\\Autoloader\" is deprecated. Use native lazy objects instead.".to_owned()
        });

        let Some(class_name_relative_to_proxy_namespace) = class_name.strip_prefix(proxy_namespace)
        else {
            return Err(Thrown::new(
                NOT_A_PROXY_CLASS,
                format!(
                    "The class \"{class_name}\" is not part of the proxy namespace \"{proxy_namespace}\""
                ),
                0,
            ));
        };

        // remove namespace separators from remaining class name
        let file_name = class_name_relative_to_proxy_namespace.replace('\\', "");

        Ok(format!(
            "{proxy_dir}{}{file_name}.php",
            std::path::MAIN_SEPARATOR
        ))
    }
}
