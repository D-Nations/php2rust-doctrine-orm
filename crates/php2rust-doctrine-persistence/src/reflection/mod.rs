//! `Doctrine\Persistence\Reflection`: the reflection properties the
//! metadata layer reads and writes values with.
//!
//! Upstream: none

mod runtime_reflection_property;
mod typed_no_default_reflection_property;

pub use runtime_reflection_property::RuntimeReflectionProperty;
pub use typed_no_default_reflection_property::TypedNoDefaultReflectionProperty;
