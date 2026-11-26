//! An object of a class declared in persistence's class registry
//! (`php_reflection::register_class`), standing in for the instances of
//! the PHP classes the ORM maps: its properties are those the class and
//! its parents declare, initialized to their defaults (typed properties
//! without a default stay uninitialized), as
//! `Instantiator::instantiate()` creates an object without calling its
//! constructor. Reflection reads and writes them through
//! `php_reflection::ReflectionObject`.
//!
//! A class and a parent that both declare a private property of the same
//! name share one slot here (PHP keeps both).
//!
//! Upstream: none

use std::sync::{Arc, PoisonError, RwLock};

use php_value::{Object, PhpArray, PhpObject, Thrown, Value};
use php2rust_doctrine_collections::reflection::RawProperties;
use php2rust_doctrine_persistence::php_reflection::{
    self, ReflectionObject, ReflectionProperty, register_reflection_object,
};
use php2rust_doctrine_persistence::proxy::Proxy;

/// The proxy state of an object whose class implements
/// `Doctrine\ORM\Proxy\InternalProxy`.
#[derive(Debug, Default)]
struct ProxyState {
    initialized: bool,
    /// How many times `__setInitialized()` was called.
    set_initialized_calls: u64,
}

/// An instance of a registered class.
#[derive(Debug)]
pub struct EntityObject {
    class: String,
    /// The declared properties; `None` is an uninitialized typed
    /// property.
    properties: RwLock<Vec<(String, Option<Value>)>>,
    proxy: Option<RwLock<ProxyState>>,
}

/// `Doctrine\ORM\Proxy\InternalProxy`.
pub const INTERNAL_PROXY: &str = "Doctrine\\ORM\\Proxy\\InternalProxy";

impl EntityObject {
    /// `(new Instantiator())->instantiate($class)`: `None` when the class
    /// is not declared.
    pub fn instantiate(class: &str) -> Option<Arc<Self>> {
        register_reflection_object::<Self>();
        let info = php_reflection::class_info(class)?;
        let mut properties: Vec<(String, Option<Value>)> = Vec::new();
        let mut current = Some(info.clone());
        let mut depth = 0;
        while let Some(class_info) = current {
            for property in class_info.properties() {
                let name = property.name();
                if properties.iter().any(|(known, _)| known == name) {
                    continue;
                }
                let Ok(reflection) = ReflectionProperty::new(class_info.name(), name) else {
                    continue;
                };
                if reflection.is_static() {
                    continue;
                }
                let value = reflection
                    .has_default_value()
                    .then(|| reflection.get_default_value())
                    .or_else(|| (!reflection.has_type()).then_some(Value::Null));
                properties.push((name.to_owned(), value));
            }
            depth += 1;
            current = class_info
                .parent()
                .filter(|_| depth < 64)
                .and_then(php_reflection::class_info);
        }
        let proxy = php_reflection::is_a(info.name(), INTERNAL_PROXY)
            .then(|| RwLock::new(ProxyState::default()));
        Some(Arc::new(Self {
            class: info.name().to_owned(),
            properties: RwLock::new(properties),
            proxy,
        }))
    }

    /// As a `php_value::Object`.
    pub fn object(class: &str) -> Option<Object> {
        Self::instantiate(class).map(|object| object as Object)
    }

    /// The value of an initialized property.
    pub fn get(&self, name: &str) -> Option<Value> {
        self.properties
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|(known, _)| known == name)
            .and_then(|(_, value)| value.clone())
    }

    /// Whether a declared property is initialized.
    pub fn is_initialized(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// The proxy's initialized flag and the number of
    /// `__setInitialized()` calls.
    pub fn proxy_state(&self) -> Option<(bool, u64)> {
        self.proxy.as_ref().map(|state| {
            let state = state.read().unwrap_or_else(PoisonError::into_inner);
            (state.initialized, state.set_initialized_calls)
        })
    }

    fn write(&self, name: &str, value: Option<Value>) -> Option<Result<(), Thrown>> {
        let mut properties = self
            .properties
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let slot = properties.iter_mut().find(|(known, _)| known == name)?;
        slot.1 = value;
        Some(Ok(()))
    }
}

impl PhpObject for EntityObject {
    fn class(&self) -> &str {
        &self.class
    }

    fn properties(&self) -> PhpArray {
        let mut array = PhpArray::new();
        for (name, value) in self
            .properties
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
        {
            if let Some(value) = value {
                array.insert(name.as_str(), value.clone());
            }
        }
        array
    }

    fn instance_of(&self, class: &str) -> bool {
        php_reflection::is_a(&self.class, class)
    }
}

impl RawProperties for EntityObject {
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>> {
        let properties = self
            .properties
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let (_, value) = properties.iter().find(|(known, _)| known == name)?;
        Some(value.clone().ok_or_else(|| {
            Thrown::new(
                "Error",
                format!(
                    "Typed property {}::${name} must not be accessed before initialization",
                    self.class
                ),
                0,
            )
        }))
    }
}

impl ReflectionObject for EntityObject {
    fn set_raw_property(&self, name: &str, value: Value) -> Option<Result<(), Thrown>> {
        self.write(name, Some(value))
    }

    fn unset_raw_property(&self, name: &str) -> Option<Result<(), Thrown>> {
        self.write(name, None)
    }

    fn as_proxy(&self) -> Option<&dyn Proxy> {
        self.proxy.as_ref().map(|_| self as &dyn Proxy)
    }
}

impl Proxy for EntityObject {
    fn load(&self) -> Result<(), Thrown> {
        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.proxy.as_ref().is_some_and(|state| {
            state
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .initialized
        })
    }

    fn set_initialized(&self, initialized: bool) -> Option<()> {
        let state = self.proxy.as_ref()?;
        let mut state = state.write().unwrap_or_else(PoisonError::into_inner);
        state.initialized = initialized;
        state.set_initialized_calls += 1;
        Some(())
    }
}
