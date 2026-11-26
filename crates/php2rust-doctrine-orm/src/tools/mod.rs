//! `Doctrine\ORM\Tools`: the schema tool and the paginator.
//!
//! Upstream: none

pub mod event;
pub mod pagination;
pub mod schema_tool;
pub mod tool_events;

pub use schema_tool::SchemaTool;
