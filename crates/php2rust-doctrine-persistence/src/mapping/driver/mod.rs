//! `Doctrine\Persistence\Mapping\Driver`: the mapping drivers and the
//! locators of mapping files and classes.
//!
//! Upstream: none

mod default_file_locator;
mod file_locator;
pub mod files;
mod mapping_driver;
mod mapping_driver_chain;
mod symfony_file_locator;

pub use default_file_locator::DefaultFileLocator;
pub use file_locator::FileLocator;
pub use mapping_driver::MappingDriver;
pub use mapping_driver_chain::MappingDriverChain;
pub use symfony_file_locator::SymfonyFileLocator;
