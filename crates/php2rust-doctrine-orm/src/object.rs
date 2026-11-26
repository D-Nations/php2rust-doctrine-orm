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

use std::sync::atomic::{AtomicBool, Ordering};
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
    /// PHP 8.4's lazy ghost state (`ReflectionClass::newLazyGhost()`):
    /// `true` while the object is an uninitialized lazy object.
    lazy: AtomicBool,
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
            lazy: AtomicBool::new(false),
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

    /// Writes a declared property (`$object->name = $value`): `false` when
    /// the class declares no such property.
    pub fn set(&self, name: &str, value: Value) -> bool {
        self.write(name, Some(value)).is_some()
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

    /// `ReflectionClass::isUninitializedLazyObject()`.
    pub fn is_uninitialized_lazy_object(&self) -> bool {
        self.lazy.load(Ordering::SeqCst)
    }

    /// Makes the object an uninitialized lazy ghost (`newLazyGhost()`), or
    /// marks it initialized (`markLazyObjectAsInitialized()`).
    pub fn set_lazy(&self, lazy: bool) {
        self.lazy.store(lazy, Ordering::SeqCst);
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

/// The body of a method the ORM calls on an entity (a lifecycle
/// callback, `$entity->$callback($eventArgs)`): what a compiler would
/// emit for the PHP method. It gets the entity and the event arguments.
pub type MethodBody = Arc<
    dyn Fn(&Object, &dyn php2rust_doctrine_event_manager::EventArgs) -> Result<(), Thrown>
        + Send
        + Sync,
>;

static METHODS: RwLock<Vec<(String, String, MethodBody)>> = RwLock::new(Vec::new());

/// Declares the body of `$class::$method()`.
pub fn register_method(class: &str, method: &str, body: MethodBody) {
    let mut methods = METHODS.write().unwrap_or_else(PoisonError::into_inner);
    methods.retain(|(c, m, _)| !(c == class && m.eq_ignore_ascii_case(method)));
    methods.push((class.to_owned(), method.to_owned(), body));
}

/// The body of `$class::$method()`, looked up through the parent classes
/// (method names are case-insensitive).
pub fn find_method(class: &str, method: &str) -> Option<MethodBody> {
    let methods = METHODS.read().unwrap_or_else(PoisonError::into_inner);
    let mut current = Some(class.to_owned());
    let mut depth = 0;
    while let Some(name) = current {
        if let Some((_, _, body)) = methods
            .iter()
            .find(|(c, m, _)| *c == name && m.eq_ignore_ascii_case(method))
        {
            return Some(body.clone());
        }
        depth += 1;
        current = php_reflection::class_info(&name)
            .and_then(|info| info.parent().map(str::to_owned))
            .filter(|_| depth < 64);
    }
    None
}

/// `$object->$method($eventArgs)`: PHP's `Error` for an undefined method.
pub fn call_method(
    object: &Object,
    method: &str,
    args: &dyn php2rust_doctrine_event_manager::EventArgs,
) -> Result<(), Thrown> {
    match find_method(object.class(), method) {
        Some(body) => body(object, args),
        None => Err(Thrown::new(
            "Error",
            format!("Call to undefined method {}::{method}()", object.class()),
            0,
        )),
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

/// The body of an entity listener's method
/// (`$listener->$method($entity, $eventArgs)`): it gets the listener
/// instance, the entity and the event arguments.
pub type ListenerMethodBody = Arc<
    dyn Fn(&Object, &Object, &dyn php2rust_doctrine_event_manager::EventArgs) -> Result<(), Thrown>
        + Send
        + Sync,
>;

static LISTENER_METHODS: RwLock<Vec<(String, String, ListenerMethodBody)>> =
    RwLock::new(Vec::new());

/// Declares the body of the entity listener method `$class::$method()`.
pub fn register_listener_method(class: &str, method: &str, body: ListenerMethodBody) {
    let mut methods = LISTENER_METHODS
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    methods.retain(|(c, m, _)| !(c == class && m.eq_ignore_ascii_case(method)));
    methods.push((class.to_owned(), method.to_owned(), body));
}

/// `$listener->$method($entity, $eventArgs)`.
pub fn call_listener_method(
    listener: &Object,
    method: &str,
    entity: &Object,
    args: &dyn php2rust_doctrine_event_manager::EventArgs,
) -> Result<(), Thrown> {
    let body = {
        let methods = LISTENER_METHODS
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        let mut current = Some(listener.class().to_owned());
        let mut found = None;
        let mut depth = 0;
        while let Some(name) = current {
            if let Some((_, _, body)) = methods
                .iter()
                .find(|(c, m, _)| *c == name && m.eq_ignore_ascii_case(method))
            {
                found = Some(body.clone());
                break;
            }
            depth += 1;
            current = php_reflection::class_info(&name)
                .and_then(|info| info.parent().map(str::to_owned))
                .filter(|_| depth < 64);
        }
        found
    };
    match body {
        Some(body) => body(listener, entity, args),
        None => Err(Thrown::new(
            "Error",
            format!("Call to undefined method {}::{method}()", listener.class()),
            0,
        )),
    }
}
