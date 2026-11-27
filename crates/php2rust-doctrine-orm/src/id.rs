//! The identifier generators `ClassMetadata::$idGenerator` holds. The ORM's
//! `Doctrine\ORM\Id` classes generate values through the entity manager,
//! which this port does not have yet: here they are the generator's class
//! and its settings, as `ClassMetadataFactory` configures them.
//!
//! `AssignedGenerator` generates through the unit of work.
//!
//! Upstream: doctrine/orm src/Id/AssignedGenerator.php

/// `Doctrine\ORM\Id\AbstractIdGenerator` and its subclasses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdGenerator {
    /// `Doctrine\ORM\Id\AssignedGenerator`.
    Assigned,
    /// `Doctrine\ORM\Id\IdentityGenerator`.
    Identity,
    /// `Doctrine\ORM\Id\BigIntegerIdentityGenerator`.
    BigIntegerIdentity,
    /// `Doctrine\ORM\Id\SequenceGenerator`.
    Sequence {
        sequence_name: String,
        allocation_size: i64,
    },
    /// A custom generator class (`#[CustomIdGenerator]`).
    Custom { class: String },
}

impl IdGenerator {
    /// The generator's class name.
    pub fn class_name(&self) -> &str {
        match self {
            Self::Assigned => "Doctrine\\ORM\\Id\\AssignedGenerator",
            Self::Identity => "Doctrine\\ORM\\Id\\IdentityGenerator",
            Self::BigIntegerIdentity => "Doctrine\\ORM\\Id\\BigIntegerIdentityGenerator",
            Self::Sequence { .. } => "Doctrine\\ORM\\Id\\SequenceGenerator",
            Self::Custom { class } => class,
        }
    }

    /// `isPostInsertGenerator()`.
    pub fn is_post_insert_generator(&self) -> bool {
        matches!(self, Self::Identity | Self::BigIntegerIdentity)
    }
}

/// Special generator for application-assigned identifiers (doesn't
/// really generate anything).
#[derive(Debug, Clone, Copy, Default)]
pub struct AssignedGenerator;

impl AssignedGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Returns the identifier assigned to the given entity.
    pub fn generate_id(
        &self,
        em: &crate::entity_manager::EntityManager,
        entity: &php_value::Object,
    ) -> Result<php_value::PhpArray, php_value::Thrown> {
        em.get_unit_of_work().generate_assigned_id(em, entity)
    }
}
