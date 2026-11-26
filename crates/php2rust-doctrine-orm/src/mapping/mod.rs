//! `Doctrine\ORM\Mapping`: class metadata, the mapping value objects,
//! the attributes and the attribute driver, naming and quote strategies.
//!
//! Upstream: none

use std::sync::Once;

use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

pub mod association_mapping;
pub mod attributes;
pub mod builder;
pub mod class_metadata;
pub mod class_metadata_factory;
pub mod driver;
pub mod embedded_class_mapping;
pub mod entity_listener_resolver;
pub mod field_mapping;
pub mod join_column_mapping;
pub mod mapping_exception;
pub mod naming_strategy;
pub mod ordered_map;
pub mod property_accessors;
pub mod property_value;
pub mod quote_strategy;
pub mod reflection_property;
pub mod typed_field_mapper;

pub use association_mapping::{
    AbstractBase, AssociationClass, AssociationKind, AssociationMapping,
};
pub use class_metadata::{ClassMetadata, ClassMetadataInfo};
pub use class_metadata_factory::{ClassMetadataFactory, FactoryConfiguration};
pub use embedded_class_mapping::{ColumnPrefix, DiscriminatorColumnMapping, EmbeddedClassMapping};
pub use field_mapping::{FieldMapping, MappingProperties};
pub use join_column_mapping::{JoinColumnMapping, JoinTableMapping};
pub use naming_strategy::{DefaultNamingStrategy, NamingStrategy, UnderscoreNamingStrategy};
pub use property_value::{Sleep, serialize_object};
pub use quote_strategy::{AnsiQuoteStrategy, DefaultQuoteStrategy, QuoteStrategy};
pub use reflection_property::{ReflectionEmbeddedProperty, ReflectionReadonlyProperty};

/// Declares the ORM's own classes in persistence's class registry (the
/// exception classes and the interfaces reflection asks about).
pub fn register_orm_classes() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        register_class(
            ClassInfo::interface("Doctrine\\ORM\\Exception\\ORMException").implements("Throwable"),
        );
        register_class(
            ClassInfo::class(mapping_exception::CLASS)
                .extends(php2rust_doctrine_persistence::mapping::mapping_exception::CLASS)
                .implements("Doctrine\\ORM\\Exception\\ORMException"),
        );
        for class in [
            mapping_exception::INVALID_CUSTOM_GENERATOR,
            mapping_exception::UNKNOWN_GENERATOR_TYPE,
        ] {
            register_class(
                ClassInfo::class(class)
                    .extends("LogicException")
                    .implements("Doctrine\\ORM\\Exception\\ORMException"),
            );
        }
        register_class(ClassInfo::class("OutOfRangeException").extends("LogicException"));
        // PHP's date classes, which parameter values and mapped fields
        // hold (`isTransient()` reflects on them).
        register_class(ClassInfo::interface("DateTimeInterface").internal());
        for class in ["DateTime", "DateTimeImmutable"] {
            register_class(
                ClassInfo::class(class)
                    .internal()
                    .implements("DateTimeInterface"),
            );
        }
        crate::query::query_exception::register_classes();
        crate::query::filter::register_classes();
        crate::abstract_query::register_classes();
        crate::cache::exception::register_classes();
    });
}
