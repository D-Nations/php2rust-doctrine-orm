//! `Doctrine\ORM\Mapping\Driver\AttributeReader`: a class's, property's or
//! method's mapping attributes, the repeatable ones gathered in a
//! `RepeatableAttributeCollection`.
//!
//! Upstream: doctrine/orm src/Mapping/Driver/AttributeReader.php src/Mapping/Driver/RepeatableAttributeCollection.php

use php_value::Thrown;
use php2rust_doctrine_persistence::php_reflection::{ReflectionClass, ReflectionProperty};

use crate::mapping::attributes::{self, AttributeClass, MappingAttribute};

/// One entry of the reader's result: an attribute, or the
/// `RepeatableAttributeCollection` of a repeatable one.
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeInstance {
    Single(MappingAttribute),
    Collection(Vec<MappingAttribute>),
}

impl AttributeInstance {
    /// `$a instanceof RepeatableAttributeCollection ? $a[0] : $a`.
    pub fn first(&self) -> Option<&MappingAttribute> {
        match self {
            Self::Single(attribute) => Some(attribute),
            Self::Collection(attributes) => attributes.first(),
        }
    }
}

/// `class-string-map<T, T|RepeatableAttributeCollection<T>>`, keyed by
/// the attribute's class name in the order first seen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttributeSet {
    entries: Vec<(String, AttributeInstance)>,
}

impl AttributeSet {
    /// `isset($attributes[$class])`.
    pub fn contains(&self, class: &str) -> bool {
        self.entry(class).is_some()
    }

    /// `$attributes[$class]`.
    pub fn entry(&self, class: &str) -> Option<&AttributeInstance> {
        self.entries
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(class))
            .map(|(_, instance)| instance)
    }

    /// The non-repeatable attribute `T`.
    pub fn get<T: AttributeClass>(&self) -> Option<&T> {
        match self.entry(T::CLASS)? {
            AttributeInstance::Single(attribute) => T::from_attribute(attribute),
            AttributeInstance::Collection(_) => None,
        }
    }

    /// The repeatable attributes `T`, empty when there are none.
    pub fn collection<T: AttributeClass>(&self) -> Vec<&T> {
        match self.entry(T::CLASS) {
            Some(AttributeInstance::Collection(attributes)) => {
                attributes.iter().filter_map(T::from_attribute).collect()
            }
            Some(AttributeInstance::Single(attribute)) => {
                T::from_attribute(attribute).into_iter().collect()
            }
            None => Vec::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &AttributeInstance)> {
        self.entries
            .iter()
            .map(|(name, instance)| (name.as_str(), instance))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// `Doctrine\ORM\Mapping\Driver\AttributeReader`.
#[derive(Debug, Clone, Copy, Default)]
pub struct AttributeReader;

impl AttributeReader {
    pub fn new() -> Self {
        Self
    }

    /// `getClassAttributes()`.
    pub fn get_class_attributes(&self, class: &ReflectionClass) -> AttributeSet {
        convert_to_attribute_instances(&attributes::class_attributes(class.get_name()))
    }

    /// `getMethodAttributes()`: `class` is the class that declares the
    /// method.
    pub fn get_method_attributes(&self, class: &str, method: &str) -> AttributeSet {
        convert_to_attribute_instances(&attributes::method_attributes(class, method))
    }

    /// `getPropertyAttributes()`.
    pub fn get_property_attributes(&self, property: &ReflectionProperty) -> AttributeSet {
        convert_to_attribute_instances(&attributes::property_attributes(
            property.class(),
            property.get_name(),
        ))
    }

    /// `getPropertyAttribute()`.
    pub fn get_property_attribute<T: AttributeClass + Clone>(
        &self,
        property: &ReflectionProperty,
    ) -> Result<Option<T>, Thrown> {
        if T::REPEATABLE {
            return Err(Thrown::new(
                "LogicException",
                format!(
                    "The attribute \"{}\" is repeatable. Call getPropertyAttributeCollection() instead.",
                    T::CLASS
                ),
                0,
            ));
        }
        Ok(self.get_property_attributes(property).get::<T>().cloned())
    }

    /// `getPropertyAttributeCollection()`.
    pub fn get_property_attribute_collection<T: AttributeClass + Clone>(
        &self,
        property: &ReflectionProperty,
    ) -> Result<Vec<T>, Thrown> {
        if !T::REPEATABLE {
            return Err(Thrown::new(
                "LogicException",
                format!(
                    "The attribute \"{}\" is not repeatable. Call getPropertyAttribute() instead.",
                    T::CLASS
                ),
                0,
            ));
        }
        Ok(self
            .get_property_attributes(property)
            .collection::<T>()
            .into_iter()
            .cloned()
            .collect())
    }
}

fn convert_to_attribute_instances(attributes: &[MappingAttribute]) -> AttributeSet {
    let mut set = AttributeSet::default();
    for attribute in attributes {
        let name = attribute.class_name().to_owned();
        let instance = attributes::new_instance(attribute);
        if attribute.is_repeatable() {
            match set.entries.iter_mut().find(|(known, _)| *known == name) {
                Some((_, AttributeInstance::Collection(collection))) => collection.push(instance),
                _ => set
                    .entries
                    .push((name, AttributeInstance::Collection(vec![instance]))),
            }
        } else {
            match set.entries.iter_mut().find(|(known, _)| *known == name) {
                Some((_, slot)) => *slot = AttributeInstance::Single(instance),
                None => set
                    .entries
                    .push((name, AttributeInstance::Single(instance))),
            }
        }
    }
    set
}
