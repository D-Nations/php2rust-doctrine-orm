//! `Doctrine\Persistence\Mapping\Driver`: the mapping drivers and the
//! locators of mapping files and classes.
//!
//! Upstream: none

mod class_locator;
mod class_names;
mod colocated_mapping_driver;
mod default_file_locator;
mod file_class_locator;
mod file_driver;
mod file_locator;
pub mod files;
mod mapping_driver;
mod mapping_driver_chain;
mod static_php_driver;
mod symfony_file_locator;

pub use class_locator::ClassLocator;
pub use class_names::ClassNames;
pub use colocated_mapping_driver::ColocatedMappingDriver;
pub use default_file_locator::DefaultFileLocator;
pub use file_class_locator::FileClassLocator;
pub use file_driver::{FileDriver, FileDriverImpl, FileDriverLocator};
pub use file_locator::FileLocator;
pub use mapping_driver::MappingDriver;
pub use mapping_driver_chain::MappingDriverChain;
pub use static_php_driver::{StaticPHPDriver, StaticPHPDriverPaths};
pub use symfony_file_locator::SymfonyFileLocator;
