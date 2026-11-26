//! Port of doctrine/collections 3.1.0: `ArrayCollection`, the
//! `Collection` interfaces, `AbstractLazyCollection`, and `Criteria` with
//! its expressions and the `ClosureExpressionVisitor` that applies them.
//! Elements are PHP values (`php_value::Value`) under PHP array keys, in
//! PHP's order; PHP closures are Rust closures over those values, and
//! what PHP throws is a `php_value::Thrown`.
//!
//! Upstream: none

#![forbid(unsafe_code)]

mod criteria;
pub mod deprecations;
pub mod expr;
mod expression_builder;
mod order;
mod php;
pub mod reflection;
mod sort_direction;

pub use criteria::{Criteria, Direction};
pub use expression_builder::ExpressionBuilder;
pub use order::Order;
pub use sort_direction::SortDirection;
