//! The part of PHP's Reflection API that `ClosureExpressionVisitor` uses:
//! `new ReflectionClass($object)`, `hasProperty()` up the parent classes,
//! and `ReflectionProperty::getRawValue()`, which reads a property of any
//! visibility without running its `get` hook.
//!
//! `php_value::PhpObject` lists public properties only, so a class whose
//! non-public (or hooked) properties must be readable registers here, once,
//! with [`register_raw_properties`]. An object of a class that is not
//! registered is read through [`PhpObject::properties`]: its public
//! properties, which is what reflection finds for a class without other
//! properties. Reflection does not see dynamic properties, so a
//! `stdClass` has none (as recorded); nor do `ArrayObject` and
//! `ArrayIterator` (whose `(array)` form is their storage).
//!
//! Upstream: none

use std::any::{Any, TypeId};
use std::sync::{PoisonError, RwLock};

use php_value::object::{ArrayIterator, ArrayObject, StdClass};
use php_value::{Key, PhpObject, Thrown, Value};

/// A class whose properties reflection reads (see the module docs).
pub trait RawProperties: PhpObject {
    /// `ReflectionClass::hasProperty($name)` on the object's class or a
    /// parent class (any visibility, and the object's dynamic properties),
    /// then `getRawValue($this)` of the first class that has it: `None`
    /// when no class has the property; `Some(Err(...))` where
    /// `getRawValue()` throws (an uninitialized typed property, a virtual
    /// property).
    fn raw_property(&self, name: &str) -> Option<Result<Value, Thrown>>;
}

type Reader = fn(&dyn PhpObject, &str) -> Option<Result<Value, Thrown>>;

static REGISTRY: RwLock<Vec<(TypeId, Reader)>> = RwLock::new(Vec::new());

fn read<T: RawProperties>(object: &dyn PhpObject, name: &str) -> Option<Result<Value, Thrown>> {
    object.downcast_ref::<T>()?.raw_property(name)
}

/// Makes the properties of `T` readable through [`RawProperties`].
/// Registering a class again does nothing.
pub fn register_raw_properties<T: RawProperties>() {
    let id = TypeId::of::<T>();
    let mut registry = REGISTRY.write().unwrap_or_else(PoisonError::into_inner);
    if registry.iter().all(|(known, _)| *known != id) {
        registry.push((id, read::<T>));
    }
}

/// The raw value of the property `name` of `object` as reflection reads
/// it; `None` when its class (and parents) have no such property.
pub fn raw_property(object: &dyn PhpObject, name: &str) -> Option<Result<Value, Thrown>> {
    let id = (object as &dyn Any).type_id();
    let reader = REGISTRY
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .find(|(known, _)| *known == id)
        .map(|(_, reader)| *reader);
    if let Some(reader) = reader {
        return reader(object, name);
    }
    if object.downcast_ref::<ArrayObject>().is_some()
        || object.downcast_ref::<ArrayIterator>().is_some()
        || object.downcast_ref::<StdClass>().is_some()
    {
        return None;
    }
    object.properties().get(&Key::from(name)).cloned().map(Ok)
}
