//! `Doctrine\ORM\Query\Filter`: the base of the user defined SQL filters,
//! its parameters and its exception.
//!
//! PHP's abstract `SQLFilter` is the [`SqlFilter`] struct: it keeps the
//! parameters and the entity manager, and holds the subclass's
//! `addFilterConstraint()` as a [`FilterConstraint`]. The configuration
//! names a filter by its class, so each filter class registers its
//! constructor ([`register_filter_class`]) for `FilterCollection::enable()`.
//!
//! Upstream: doctrine/orm src/Query/Filter/SQLFilter.php src/Query/Filter/Parameter.php
//! Upstream: doctrine/orm src/Query/Filter/FilterException.php

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, RwLock, Weak};

use php_value::object::EnumCase;
use php_value::{PhpArray, PhpString, Thrown, Value};
use php2rust_doctrine_dbal::types::Types;
use php2rust_doctrine_dbal::{Connection, ConnectionStandIn, WrapperParameterType};

use crate::entity_manager::EntityManager;
use crate::exception::from_dbal;
use crate::mapping::ClassMetadataInfo;
use crate::query::ParameterTypeInferer;
use crate::query::context::SqlFilterConstraint;

/// `Doctrine\ORM\Query\Filter\SQLFilter`.
pub const CLASS: &str = "Doctrine\\ORM\\Query\\Filter\\SQLFilter";

/// `Doctrine\ORM\Query\Filter\FilterException`.
pub const FILTER_EXCEPTION: &str = "Doctrine\\ORM\\Query\\Filter\\FilterException";

/// `FilterException::cannotConvertListParameterIntoSingleValue($name)`.
pub fn cannot_convert_list_parameter_into_single_value(name: &str) -> Thrown {
    Thrown::new(
        FILTER_EXCEPTION,
        format!("Cannot convert list-based SQL filter parameter \"{name}\" into a single value."),
        0,
    )
}

/// `FilterException::cannotConvertSingleParameterIntoListValue($name)`.
pub fn cannot_convert_single_parameter_into_list_value(name: &str) -> Thrown {
    Thrown::new(
        FILTER_EXCEPTION,
        format!("Cannot convert single SQL filter parameter \"{name}\" into a list value."),
        0,
    )
}

/// Declares the filter exception class in persistence's class registry.
pub(crate) fn register_classes() {
    use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

    register_class(
        ClassInfo::class(FILTER_EXCEPTION)
            .extends("LogicException")
            .implements("Doctrine\\ORM\\Exception\\ORMException"),
    );
}

/// `Doctrine\ORM\Query\Filter\Parameter` (internal).
#[derive(Debug, Clone)]
pub struct Parameter {
    pub value: Value,
    pub type_: WrapperParameterType,
    pub is_list: bool,
}

/// A parameter type as PHP holds it: a type name, or a `ParameterType` or
/// `ArrayParameterType` case.
fn type_value(type_: &WrapperParameterType) -> Value {
    match type_ {
        WrapperParameterType::Name(name) => Value::String(name.clone()),
        WrapperParameterType::Parameter(case) => Value::Object(
            EnumCase::new("Doctrine\\DBAL\\ParameterType", case.name(), None).object(),
        ),
        WrapperParameterType::Array(case) => Value::Object(
            EnumCase::new("Doctrine\\DBAL\\ArrayParameterType", case.name(), None).object(),
        ),
        // Not reached: the filter API takes type names.
        WrapperParameterType::Type(type_) => Value::string(type_.class()),
    }
}

/// `(string) $value` (strict types): arrays give "Array" (PHP warns), an
/// object that is not `Stringable` fails.
fn to_string(value: &Value) -> Result<PhpString, Thrown> {
    match value {
        Value::Array(_) => Ok(PhpString::from("Array")),
        Value::Object(object) => object.to_php_string().ok_or_else(|| {
            Thrown::new(
                "Error",
                format!(
                    "Object of class {} could not be converted to string",
                    object.class()
                ),
                0,
            )
        }),
        other => Ok(other.to_php_string().unwrap_or_default()),
    }
}

/// A subclass's `addFilterConstraint()`.
pub trait FilterConstraint: Send + Sync {
    /// Gets the SQL query part to add to a query: the constraint SQL if
    /// there is available, empty string otherwise.
    fn add_filter_constraint(
        &self,
        filter: &SqlFilter,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown>;
}

/// `new $filterClass($em)`: the constructor a filter class registers.
pub type FilterFactory = Arc<dyn Fn(&Arc<EntityManager>) -> SqlFilter + Send + Sync>;

fn filter_classes() -> &'static RwLock<HashMap<String, FilterFactory>> {
    static CLASSES: OnceLock<RwLock<HashMap<String, FilterFactory>>> = OnceLock::new();
    CLASSES.get_or_init(Default::default)
}

/// Registers the constructor of a filter class, for the class names an
/// `EntityManager`'s `Configuration` holds (`addFilter()`).
pub fn register_filter_class(class_name: &str, factory: FilterFactory) {
    filter_classes()
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(class_name.trim_start_matches('\\').to_owned(), factory);
}

/// `new $filterClass($em)` for a registered filter class.
pub(crate) fn new_filter(class_name: &str, em: &Arc<EntityManager>) -> Result<SqlFilter, Thrown> {
    let factory = filter_classes()
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(class_name.trim_start_matches('\\'))
        .cloned();
    match factory {
        Some(factory) => Ok(factory(em)),
        None => Err(Thrown::new(
            "Error",
            format!("Class \"{class_name}\" not found"),
            0,
        )),
    }
}

/// The base class that user defined filters should extend.
///
/// Handles the setting and escaping of parameters.
pub struct SqlFilter {
    class: String,
    em: Weak<EntityManager>,
    /// Parameters for the filter, sorted by name.
    parameters: Mutex<BTreeMap<String, Parameter>>,
    constraint: Box<dyn FilterConstraint>,
}

impl std::fmt::Debug for SqlFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqlFilter")
            .field("class", &self.class)
            .field("parameters", &*self.params())
            .finish_non_exhaustive()
    }
}

impl SqlFilter {
    /// `new $class($em)`: the filter of class `class` (a subclass of
    /// `SQLFilter`) whose `addFilterConstraint()` is `constraint`.
    pub fn new(
        em: &Arc<EntityManager>,
        class: &str,
        constraint: Box<dyn FilterConstraint>,
    ) -> Self {
        Self {
            class: class.trim_start_matches('\\').to_owned(),
            em: Arc::downgrade(em),
            parameters: Mutex::new(BTreeMap::new()),
            constraint,
        }
    }

    /// `get_class($filter)`.
    pub fn class(&self) -> &str {
        &self.class
    }

    fn params(&self) -> MutexGuard<'_, BTreeMap<String, Parameter>> {
        self.parameters
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn em(&self) -> Result<Arc<EntityManager>, Thrown> {
        self.em
            .upgrade()
            .ok_or_else(|| Thrown::new("Error", "The entity manager of the filter is gone", 0))
    }

    /// Sets a parameter list that can be used by the filter.
    ///
    /// `$type`: the parameter type (`Types::STRING` by default). If
    /// specified, the given value will be run through the type conversion
    /// of this type.
    pub fn set_parameter_list(
        &self,
        name: &str,
        values: PhpArray,
        type_: Option<&str>,
    ) -> Result<&Self, Thrown> {
        self.params().insert(
            name.to_owned(),
            Parameter {
                value: Value::Array(values),
                type_: type_.unwrap_or(Types::STRING).into(),
                is_list: true,
            },
        );

        // The filter collection of the EM is now dirty
        self.em()?.get_filters().set_filters_state_dirty();

        Ok(self)
    }

    /// Sets a parameter that can be used by the filter.
    ///
    /// `$type`: the parameter type. If specified, the given value will be
    /// run through the type conversion of this type. This is usually not
    /// needed for strings and numeric types.
    pub fn set_parameter(
        &self,
        name: &str,
        value: Value,
        type_: Option<&str>,
    ) -> Result<&Self, Thrown> {
        let type_ = match type_ {
            Some(type_) => type_.into(),
            None => ParameterTypeInferer::infer_type(&value),
        };
        self.params().insert(
            name.to_owned(),
            Parameter {
                value,
                type_,
                is_list: false,
            },
        );

        // The filter collection of the EM is now dirty
        self.em()?.get_filters().set_filters_state_dirty();

        Ok(self)
    }

    fn quote(&self, value: &Value) -> Result<String, Thrown> {
        let text = to_string(value)?;
        ConnectionStandIn::quote(&**self.em()?.get_connection(), &text.to_string_lossy())
            .map_err(|e| from_dbal(&e))
    }

    /// Gets a parameter to use in a query.
    ///
    /// The function is responsible for the right output escaping to use the
    /// value in a query: the SQL escaped parameter.
    pub fn get_parameter(&self, name: &str) -> Result<String, Thrown> {
        let parameter = self.params().get(name).cloned();
        let Some(parameter) = parameter else {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!("Parameter '{name}' does not exist."),
                0,
            ));
        };

        if parameter.is_list {
            return Err(cannot_convert_list_parameter_into_single_value(name));
        }

        self.quote(&parameter.value)
    }

    /// Gets a parameter to use in a query assuming it's a list of entries.
    ///
    /// The function is responsible for the right output escaping to use the
    /// value in a query, separating each entry by comma to inline it into
    /// an `IN()` query part.
    pub fn get_parameter_list(&self, name: &str) -> Result<String, Thrown> {
        let parameter = self.params().get(name).cloned();
        let Some(parameter) = parameter else {
            return Err(Thrown::new(
                "InvalidArgumentException",
                format!("Parameter '{name}' does not exist."),
                0,
            ));
        };

        if !parameter.is_list {
            return Err(cannot_convert_single_parameter_into_list_value(name));
        }

        let values: Vec<Value> = match &parameter.value {
            Value::Array(values) => values.values().cloned().collect(),
            other => vec![other.clone()],
        };
        let quoted = values
            .iter()
            .map(|value| self.quote(value))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(quoted.join(","))
    }

    /// Checks if a parameter was set for the filter.
    pub fn has_parameter(&self, name: &str) -> bool {
        self.params().contains_key(name)
    }

    /// Returns as string representation of the `SQLFilter` parameters (the
    /// state): `__toString()`.
    pub fn to_php_string(&self) -> PhpString {
        let mut parameters = PhpArray::new();
        for (name, parameter) in self.params().iter() {
            let mut entry = PhpArray::new();
            entry.insert("value", parameter.value.clone());
            entry.insert("type", type_value(&parameter.type_));
            entry.insert("is_list", Value::Bool(parameter.is_list));
            parameters.insert(name.as_str(), Value::Array(entry));
        }
        php_value::serialize::serialize(&Value::Array(parameters))
    }

    /// Returns the database connection used by the entity manager
    /// (protected upstream).
    pub fn get_connection(&self) -> Result<Arc<Connection>, Thrown> {
        Ok(self.em()?.get_connection().clone())
    }

    /// Gets the SQL query part to add to a query: the subclass's
    /// `addFilterConstraint()`.
    pub fn add_filter_constraint(
        &self,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        self.constraint
            .add_filter_constraint(self, target_entity, target_table_alias)
    }
}

impl SqlFilterConstraint for SqlFilter {
    fn add_filter_constraint(
        &self,
        target_entity: &ClassMetadataInfo,
        target_table_alias: &str,
    ) -> Result<String, Thrown> {
        Self::add_filter_constraint(self, target_entity, target_table_alias)
    }
}
