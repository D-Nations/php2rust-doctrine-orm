//! `Doctrine\ORM\Persisters`: the entity and collection persisters.
//!
//! Upstream: none

mod basic_entity_persister;
pub mod collection;

pub use basic_entity_persister::BasicEntityPersister;
