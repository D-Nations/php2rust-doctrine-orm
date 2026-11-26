//! `Doctrine\Persistence\Mapping`: class metadata, its factory, the
//! reflection service and the mapping drivers.
//!
//! Upstream: none

mod abstract_class_metadata_factory;
mod class_metadata;
mod class_metadata_factory;
pub mod driver;
pub mod mapping_exception;
mod proxy_class_name_resolver;
mod reflection_service;
mod runtime_reflection_service;

pub use abstract_class_metadata_factory::{
    AbstractClassMetadataFactory, ClassMetadataFactoryState, DEFAULT_CACHE_SALT,
    DefaultProxyClassNameResolver,
};
pub use class_metadata::ClassMetadata;
pub use class_metadata_factory::ClassMetadataFactory;
pub use proxy_class_name_resolver::ProxyClassNameResolver;
pub use reflection_service::ReflectionService;
pub use runtime_reflection_service::RuntimeReflectionService;

/// `parent::method()` of `AbstractClassMetadataFactory`.
pub mod abstract_factory {
    pub use super::abstract_class_metadata_factory::{
        get_all_metadata, get_cache_key, get_metadata_for, get_parent_classes, is_transient,
        load_metadata,
    };
}
