//! `Doctrine\ORM\Proxy\ProxyFactory`: the port only has native lazy
//! objects (PHP 8.4), so a proxy is a lazy ghost of the entity
//! (`EntityManager::get_proxy()`) and no proxy class is generated.
//!
//! Upstream: doctrine/orm src/Proxy/ProxyFactory.php

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
