//! Port of doctrine/orm 3.7.5. See README.md for the contract.
//!
//! Doctrine source copyright (c) Doctrine Project; distributed under MIT.
//!
//! Upstream: none

#![forbid(unsafe_code)]

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

pub use query_builder::QueryBuilder;
pub use query_type::QueryType;

pub mod abstract_query;
pub mod cache;
pub mod collection_object;
pub mod configuration;
pub mod deprecations;
pub mod entity_manager;
pub mod entity_repository;
pub mod event;
pub mod events;
pub mod exception;
pub mod id;
pub mod internal;
pub mod lazy_criteria_collection;
pub mod mapping;
pub mod native_query;
pub mod object;
pub mod orm_setup;
pub mod persistent_collection;
pub mod persisters;
pub mod proxy;
pub mod query;
pub mod query_builder;
pub mod query_type;
pub mod repository;
pub mod tools;
pub mod unit_of_work;
pub mod utility;
