//! An [`InsertBatch`] represents a set of entities that are safe to be
//! batched together in a single query.
//!
//! These entities are only those that have all fields already assigned,
//! including the identifier field(s).
//!
//! This data structure only exists for internal `UnitOfWork`
//! optimisations, and should not be relied upon outside the ORM.
//!
//! `batchByEntityType()` takes the entity manager's `getClassMetadata()`
//! as a function.
//!
//! Upstream: doctrine/orm src/Internal/UnitOfWork/InsertBatch.php

use std::sync::Arc;

use php_value::{Object, Thrown};

use crate::id::IdGenerator;
use crate::mapping::ClassMetadata;

/// `Doctrine\ORM\Internal\UnitOfWork\InsertBatch`.
pub struct InsertBatch {
    pub class: Arc<ClassMetadata>,
    pub entities: Vec<Object>,
}

impl InsertBatch {
    pub fn new(class: Arc<ClassMetadata>, entities: Vec<Object>) -> Self {
        Self { class, entities }
    }

    /// This method will batch the given entity set by type, preserving
    /// their order. For example, given an input [A1, A2, A3, B1, B2, A4,
    /// A5], it will create an [[A1, A2, A3], [B1, B2], [A4, A5]] batch.
    ///
    /// Entities for which the identifier needs to be generated or fetched
    /// by a sequence are put as single items in a batch of their own,
    /// since it is unsafe to batch-insert them.
    pub fn batch_by_entity_type(
        get_class_metadata: impl Fn(&str) -> Result<Arc<ClassMetadata>, Thrown>,
        entities: Vec<Object>,
    ) -> Result<Vec<Self>, Thrown> {
        let mut current_class: Option<String> = None;
        let mut batches: Vec<Self> = Vec::new();

        for entity in entities {
            let entity_class = get_class_metadata(entity.class())?;
            let (name, assigned) = {
                let info = entity_class.read();
                (
                    info.name.clone(),
                    matches!(info.id_generator, Some(IdGenerator::Assigned)),
                )
            };

            match batches.last_mut() {
                Some(batch) if current_class.as_deref() == Some(name.as_str()) && assigned => {
                    batch.entities.push(entity);
                }
                _ => {
                    current_class = Some(name);
                    batches.push(Self::new(entity_class, vec![entity]));
                }
            }
        }

        Ok(batches)
    }
}
