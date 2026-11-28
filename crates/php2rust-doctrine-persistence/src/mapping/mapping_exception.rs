//! A `MappingException` indicates that something is wrong with the mapping
//! setup.
//!
//! The exceptions are `php_value::Thrown`s of the class [`CLASS`];
//! [`is_mapping_exception`] is `instanceof MappingException`, which also
//! holds for registered subclasses (`php_reflection::register_class`).
//!
//! Upstream: doctrine/persistence src/Mapping/MappingException.php

use php_value::Thrown;

use crate::php_reflection;

/// The class name.
pub const CLASS: &str = "Doctrine\\Persistence\\Mapping\\MappingException";

fn new(message: String) -> Thrown {
    Thrown::new(CLASS, message, 0)
}

/// `$e instanceof MappingException`.
pub fn is_mapping_exception(thrown: &Thrown) -> bool {
    php_reflection::thrown_is_a(thrown, CLASS)
}

pub fn class_not_found_in_namespaces(class_name: &str, namespaces: &[String]) -> Thrown {
    new(format!(
        "The class '{class_name}' was not found in the chain configured namespaces {}",
        namespaces.join(", ")
    ))
}

pub fn path_required_for_driver(driver_class_name: &str) -> Thrown {
    new(format!(
        "Specifying source file paths to your entities is required when using {driver_class_name} to retrieve all class names."
    ))
}

pub fn file_mapping_drivers_require_configured_directory_path(path: Option<&str>) -> Thrown {
    let path = path.map_or_default(|p| format!("[{p}]"));
    new(format!(
        "File mapping drivers must have a valid directory path, however the given path {path} seems to be incorrect!"
    ))
}

pub fn mapping_file_not_found(entity_name: &str, file_name: &str) -> Thrown {
    new(format!(
        "No mapping file found named '{file_name}' for class '{entity_name}'."
    ))
}

pub fn invalid_mapping_file(entity_name: &str, file_name: &str) -> Thrown {
    new(format!(
        "Invalid mapping file '{file_name}' for class '{entity_name}'."
    ))
}

pub fn non_existing_class(class_name: &str) -> Thrown {
    new(format!("Class '{class_name}' does not exist"))
}

pub fn class_is_anonymous(class_name: &str) -> Thrown {
    new(format!("Class \"{class_name}\" is anonymous"))
}
