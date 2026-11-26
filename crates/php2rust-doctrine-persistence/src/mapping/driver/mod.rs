//! `Doctrine\Persistence\Mapping\Driver`: the mapping drivers and the
//! locators of mapping files and classes.
//!
//! Upstream: none

mod mapping_driver;
mod mapping_driver_chain;

pub use mapping_driver::MappingDriver;
pub use mapping_driver_chain::MappingDriverChain;
