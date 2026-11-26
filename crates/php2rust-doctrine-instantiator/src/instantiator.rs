//! `Instantiator`.
//!
//! Upstream: doctrine/instantiator src/Doctrine/Instantiator/Instantiator.php

use std::collections::HashMap;
use std::sync::{LazyLock, Once, PoisonError, RwLock};

use php_value::Object;
use php2rust_doctrine_persistence::php_reflection::{
    ClassInfo, ReflectionClass, class_exists, enum_exists, register_class, thrown_is_a,
};

use crate::InstantiatorInterface;
use crate::exception::{InstantiatorError, InvalidArgumentException, UnexpectedValueException};

const NAMESPACE: &str = "Doctrine\\Instantiator\\";

/// Declares the instantiator's own classes and interfaces in the class
/// registry, once (PHP knows them once their files are autoloaded).
pub fn declare_classes() {
    static DECLARED: Once = Once::new();
    DECLARED.call_once(|| {
        let exception_interface = format!("{NAMESPACE}Exception\\ExceptionInterface");
        for info in [
            ClassInfo::interface(format!("{NAMESPACE}InstantiatorInterface")),
            ClassInfo::class(format!("{NAMESPACE}Instantiator"))
                .final_class()
                .implements(format!("{NAMESPACE}InstantiatorInterface")),
            ClassInfo::interface(exception_interface.clone()).implements("Throwable"),
            ClassInfo::class(InvalidArgumentException::CLASS)
                .extends("InvalidArgumentException")
                .implements(exception_interface.clone()),
            ClassInfo::class(UnexpectedValueException::CLASS)
                .extends("UnexpectedValueException")
                .implements(exception_interface),
        ] {
            register_class(info);
        }
    });
}

/// How a cached class is instantiated.
#[derive(Debug, Clone)]
enum Factory {
    /// `ReflectionClass::newInstanceWithoutConstructor()`.
    Reflection(ReflectionClass),
    /// `unserialize()` of an empty payload: the class's blank instance.
    UnSerialization(ReflectionClass),
}

impl Factory {
    fn call(&self) -> Result<Object, InstantiatorError> {
        match self {
            Self::Reflection(class) => Ok(class.new_instance_without_constructor()?),
            Self::UnSerialization(class) => Ok(class.blank_instance()?),
        }
    }
}

/// Used to instantiate specific classes, indexed by class name.
static CACHED_INSTANTIATORS: LazyLock<RwLock<HashMap<String, Factory>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Copy)]
pub struct Instantiator {
    _private: (),
}

impl Default for Instantiator {
    fn default() -> Self {
        Self::new()
    }
}

impl Instantiator {
    pub fn new() -> Self {
        declare_classes();
        Self { _private: () }
    }

    /// Builds the requested object and caches its factory for performance
    fn build_and_cache_from_factory(class_name: &str) -> Result<Object, InstantiatorError> {
        let factory = Self::build_factory(class_name)?;
        CACHED_INSTANTIATORS
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(class_name.to_owned(), factory.clone());
        factory.call()
    }

    /// Builds a factory capable of instantiating the given `class_name`
    /// without invoking its constructor.
    fn build_factory(class_name: &str) -> Result<Factory, InstantiatorError> {
        let reflection_class = Self::get_reflection_class(class_name)?;
        if Self::is_instantiable_via_reflection(&reflection_class) {
            return Ok(Factory::Reflection(reflection_class));
        }
        Self::check_if_un_serialization_is_supported(&reflection_class)?;
        Ok(Factory::UnSerialization(reflection_class))
    }

    fn get_reflection_class(class_name: &str) -> Result<ReflectionClass, InstantiatorError> {
        if !class_exists(class_name) {
            return Err(InvalidArgumentException::from_non_existing_class(class_name).into());
        }
        if enum_exists(class_name) {
            return Err(InvalidArgumentException::from_enum(class_name).into());
        }
        let reflection = ReflectionClass::new(class_name)?;
        if reflection.is_abstract() {
            return Err(InvalidArgumentException::from_abstract_class(&reflection).into());
        }
        Ok(reflection)
    }

    /// PHP also turns a warning or notice raised while un-serializing into
    /// `UnexpectedValueException::fromUncleanUnSerialization()`; a blank
    /// instance here either is made or throws.
    fn check_if_un_serialization_is_supported(
        reflection_class: &ReflectionClass,
    ) -> Result<(), InstantiatorError> {
        match reflection_class.blank_instance() {
            Ok(_) => Ok(()),
            Err(thrown) if thrown_is_a(&thrown, "Exception") => Err(
                UnexpectedValueException::from_serialization_triggered_exception(
                    reflection_class,
                    thrown,
                )
                .into(),
            ),
            Err(thrown) => Err(thrown.into()),
        }
    }

    fn is_instantiable_via_reflection(reflection_class: &ReflectionClass) -> bool {
        !(Self::has_internal_ancestors(reflection_class) && reflection_class.is_final())
    }

    /// Verifies whether the given class is to be considered internal
    fn has_internal_ancestors(reflection_class: &ReflectionClass) -> bool {
        let mut current = Some(reflection_class.clone());
        let mut depth = 0;
        while let Some(class) = current {
            if class.is_internal() {
                return true;
            }
            depth += 1;
            if depth > 64 {
                break;
            }
            current = class.get_parent_class();
        }
        false
    }
}

impl InstantiatorInterface for Instantiator {
    fn instantiate(&self, class_name: &str) -> Result<Object, InstantiatorError> {
        let cached = CACHED_INSTANTIATORS
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(class_name)
            .cloned();
        if let Some(factory) = cached {
            return factory.call();
        }
        Self::build_and_cache_from_factory(class_name)
    }
}
