//! `Doctrine\ORM\Mapping\Driver`: the attribute driver and its reader, and
//! the XML driver.
//!
//! Upstream: none

pub mod attribute_driver;
pub mod attribute_reader;
pub mod xml_driver;

pub use attribute_driver::AttributeDriver;
pub use attribute_reader::{AttributeInstance, AttributeReader, AttributeSet};
pub use xml_driver::{XmlDriver, XmlElement};
