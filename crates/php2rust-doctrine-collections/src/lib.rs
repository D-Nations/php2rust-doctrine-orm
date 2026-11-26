//! Port of doctrine/collections 3.1.0: `ArrayCollection`, the
//! `Collection` interfaces, `AbstractLazyCollection`, and `Criteria` with
//! its expressions and the `ClosureExpressionVisitor` that applies them.
//! Elements are PHP values (`php_value::Value`) under PHP array keys, in
//! PHP's order; PHP closures are Rust closures over those values, and
//! what PHP throws is a `php_value::Thrown`.
//!
//! ```
//! use php2rust_doctrine_collections::{ArrayCollection, Criteria, Order};
//! use php_value::{Value, php_array};
//!
//! let collection = ArrayCollection::new(php_array![
//!     "a" => php_array!["name" => "b", "rank" => 2],
//!     "b" => php_array!["name" => "a", "rank" => 1],
//!     "c" => php_array!["name" => "c", "rank" => 3],
//! ]);
//!
//! let mut criteria = Criteria::create();
//! criteria.r#where(Criteria::expr().gt("rank", 1));
//! criteria.order_by([("name", Order::Ascending)]);
//!
//! let matched = collection.matching(&criteria).unwrap();
//! assert_eq!(matched.get_keys(), php_array!["a", "c"]);
//! assert_eq!(matched.count(), 2);
//! assert!(matched.contains_key(&"c".into()));
//! assert_eq!(
//!     collection.get(&"b".into()),
//!     Value::Array(php_array!["name" => "a", "rank" => 1])
//! );
//! ```
//!
//! Upstream: none

#![forbid(unsafe_code)]

mod abstract_lazy_collection;
mod array_collection;
mod collection;
mod criteria;
pub mod deprecations;
pub mod expr;
mod expression_builder;
mod order;
mod php;
mod readable_collection;
pub mod reflection;
mod selectable;
mod sort_direction;

pub use abstract_lazy_collection::{AbstractLazyCollection, DoInitialize};
pub use array_collection::{ArrayCollection, ArrayCollectionClass};
pub use collection::Collection;
pub use criteria::{Criteria, Direction};
pub use expression_builder::ExpressionBuilder;
pub use order::Order;
pub use readable_collection::{Partition, ReadableCollection};
pub use selectable::Selectable;
pub use sort_direction::SortDirection;
