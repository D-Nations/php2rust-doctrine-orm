//! `Doctrine\ORM\Tools`: the schema tool, the schema validator and the
//! paginator.
//!
//! Upstream: none

pub mod event;
pub mod pagination;
pub mod schema_tool;
pub mod schema_validator;
pub mod tool_events;

pub use schema_tool::SchemaTool;
