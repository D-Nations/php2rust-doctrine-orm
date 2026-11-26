//! `Doctrine\Persistence\Mapping`: class metadata, its factory, the
//! reflection service and the mapping drivers.
//!
//! Upstream: none

mod class_metadata;
mod class_metadata_factory;
pub mod mapping_exception;
mod reflection_service;
mod runtime_reflection_service;

pub use class_metadata::ClassMetadata;
pub use class_metadata_factory::ClassMetadataFactory;
pub use reflection_service::ReflectionService;
pub use runtime_reflection_service::RuntimeReflectionService;

