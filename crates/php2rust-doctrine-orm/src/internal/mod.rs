//! `Doctrine\ORM\Internal`: the commit order graphs and the hydration.
//!
//! Upstream: none

pub mod hydration;
mod hydration_complete_handler;
mod strongly_connected_components;
mod topological_sort;
pub mod unit_of_work;

pub use hydration_complete_handler::{HydrationCompleteHandler, PostLoadListenersInvoker};
pub use strongly_connected_components::StronglyConnectedComponents;
pub use topological_sort::{CycleDetectedException, TopologicalSort};

use php_value::Object;

/// `spl_object_id($object)`: the identity of a shared object.
pub fn object_id(object: &Object) -> usize {
    std::sync::Arc::as_ptr(object).cast::<()>() as usize
}
