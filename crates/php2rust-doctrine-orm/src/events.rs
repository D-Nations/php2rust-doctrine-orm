//! `Doctrine\ORM\Events`: the names of the ORM's events.
//!
//! Upstream: doctrine/orm src/Events.php

pub const PRE_REMOVE: &str = "preRemove";
pub const POST_REMOVE: &str = "postRemove";
pub const PRE_PERSIST: &str = "prePersist";
pub const POST_PERSIST: &str = "postPersist";
pub const PRE_UPDATE: &str = "preUpdate";
pub const POST_UPDATE: &str = "postUpdate";
pub const POST_LOAD: &str = "postLoad";
pub const LOAD_CLASS_METADATA: &str = "loadClassMetadata";
pub const ON_CLASS_METADATA_NOT_FOUND: &str = "onClassMetadataNotFound";
pub const PRE_FLUSH: &str = "preFlush";
pub const ON_FLUSH: &str = "onFlush";
pub const POST_FLUSH: &str = "postFlush";
pub const ON_CLEAR: &str = "onClear";
