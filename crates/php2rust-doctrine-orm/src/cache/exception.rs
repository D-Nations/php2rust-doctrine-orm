//! The second-level cache's exceptions, as `php_value::Thrown` (class
//! and message), with upstream's message texts.
//!
//! Upstream: doctrine/orm src/Cache/CacheException.php src/Cache/LockException.php
//! Upstream: doctrine/orm src/Cache/Exception/CacheException.php
//! Upstream: doctrine/orm src/Cache/Exception/CannotUpdateReadOnlyCollection.php
//! Upstream: doctrine/orm src/Cache/Exception/CannotUpdateReadOnlyEntity.php
//! Upstream: doctrine/orm src/Cache/Exception/FeatureNotImplemented.php
//! Upstream: doctrine/orm src/Cache/Exception/NonCacheableEntity.php
//! Upstream: doctrine/orm src/Cache/Exception/NonCacheableEntityAssociation.php

use php_value::Thrown;

pub const CACHE_EXCEPTION: &str = "Doctrine\\ORM\\Cache\\CacheException";
/// `Doctrine\ORM\Cache\Exception\CacheException`, the base of the
/// `Cache\Exception` classes.
pub const EXCEPTION_CACHE_EXCEPTION: &str = "Doctrine\\ORM\\Cache\\Exception\\CacheException";
pub const LOCK_EXCEPTION: &str = "Doctrine\\ORM\\Cache\\LockException";
pub const CANNOT_UPDATE_READ_ONLY_COLLECTION: &str =
    "Doctrine\\ORM\\Cache\\Exception\\CannotUpdateReadOnlyCollection";
pub const CANNOT_UPDATE_READ_ONLY_ENTITY: &str =
    "Doctrine\\ORM\\Cache\\Exception\\CannotUpdateReadOnlyEntity";
pub const FEATURE_NOT_IMPLEMENTED: &str = "Doctrine\\ORM\\Cache\\Exception\\FeatureNotImplemented";
pub const NON_CACHEABLE_ENTITY: &str = "Doctrine\\ORM\\Cache\\Exception\\NonCacheableEntity";
pub const NON_CACHEABLE_ENTITY_ASSOCIATION: &str =
    "Doctrine\\ORM\\Cache\\Exception\\NonCacheableEntityAssociation";
pub const INVALID_ARGUMENT_EXCEPTION: &str = "InvalidArgumentException";
pub const LOGIC_EXCEPTION: &str = "LogicException";
pub use super::pool::PSR_INVALID_ARGUMENT_EXCEPTION;

/// Declares the cache exception classes in persistence's class registry.
pub(crate) fn register_classes() {
    use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

    register_class(
        ClassInfo::class(CACHE_EXCEPTION)
            .extends(LOGIC_EXCEPTION)
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
    register_class(ClassInfo::class(LOCK_EXCEPTION).extends(CACHE_EXCEPTION));
    register_class(ClassInfo::class(EXCEPTION_CACHE_EXCEPTION).extends(CACHE_EXCEPTION));
    for class in [
        CANNOT_UPDATE_READ_ONLY_COLLECTION,
        CANNOT_UPDATE_READ_ONLY_ENTITY,
        FEATURE_NOT_IMPLEMENTED,
        NON_CACHEABLE_ENTITY,
        NON_CACHEABLE_ENTITY_ASSOCIATION,
    ] {
        register_class(ClassInfo::class(class).extends(EXCEPTION_CACHE_EXCEPTION));
    }
}

fn new(class: &str, message: String) -> Thrown {
    Thrown::new(class, message, 0)
}

/// `new LockException($message)`.
pub fn lock_exception(message: &str) -> Thrown {
    new(LOCK_EXCEPTION, message.to_owned())
}

/// `CannotUpdateReadOnlyCollection::fromEntityAndField()`.
pub fn cannot_update_read_only_collection(source_entity: &str, field_name: &str) -> Thrown {
    new(
        CANNOT_UPDATE_READ_ONLY_COLLECTION,
        format!("Cannot update a readonly collection \"{source_entity}#{field_name}\""),
    )
}

/// `CannotUpdateReadOnlyEntity::fromEntity()`.
pub fn cannot_update_read_only_entity(entity_name: &str) -> Thrown {
    new(
        CANNOT_UPDATE_READ_ONLY_ENTITY,
        format!("Cannot update a readonly entity \"{entity_name}\""),
    )
}

/// `FeatureNotImplemented::scalarResults()`.
pub fn scalar_results() -> Thrown {
    new(
        FEATURE_NOT_IMPLEMENTED,
        "Second level cache does not support scalar results.".to_owned(),
    )
}

/// `FeatureNotImplemented::multipleRootEntities()`.
pub fn multiple_root_entities() -> Thrown {
    new(
        FEATURE_NOT_IMPLEMENTED,
        "Second level cache does not support multiple root entities.".to_owned(),
    )
}

/// `FeatureNotImplemented::nonSelectStatements()`.
pub fn non_select_statements() -> Thrown {
    new(
        FEATURE_NOT_IMPLEMENTED,
        "Second-level cache query supports only select statements.".to_owned(),
    )
}

/// `FeatureNotImplemented::partialEntities()`.
pub fn partial_entities() -> Thrown {
    new(
        FEATURE_NOT_IMPLEMENTED,
        "Second level cache does not support partial entities.".to_owned(),
    )
}

/// `NonCacheableEntity::fromEntity()`.
pub fn non_cacheable_entity(entity_name: &str) -> Thrown {
    new(
        NON_CACHEABLE_ENTITY,
        format!("Entity \"{entity_name}\" not configured as part of the second-level cache."),
    )
}

/// `NonCacheableEntityAssociation::fromEntityAndField()`.
pub fn non_cacheable_entity_association(entity_name: &str, field: &str) -> Thrown {
    new(
        NON_CACHEABLE_ENTITY_ASSOCIATION,
        format!(
            "Entity association field \"{entity_name}#{field}\" not configured as part of the second-level cache."
        ),
    )
}

/// `new InvalidArgumentException($message)`.
pub fn invalid_argument(message: String) -> Thrown {
    new(INVALID_ARGUMENT_EXCEPTION, message)
}

/// `new LogicException($message)`.
pub fn logic_exception(message: String) -> Thrown {
    new(LOGIC_EXCEPTION, message)
}
