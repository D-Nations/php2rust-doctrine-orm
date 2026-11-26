//! `Doctrine\ORM\Mapping\Driver`: the attribute driver and its reader.
//!
//! Upstream: none

pub mod attribute_driver;
pub mod attribute_reader;

pub use attribute_driver::AttributeDriver;
pub use attribute_reader::{AttributeInstance, AttributeReader, AttributeSet};
