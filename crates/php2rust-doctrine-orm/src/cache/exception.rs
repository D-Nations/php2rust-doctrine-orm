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
pub const PSR_INVALID_ARGUMENT_EXCEPTION: &str =
    "Symfony\\Component\\Cache\\Exception\\InvalidArgumentException";

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
