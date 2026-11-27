//! `Doctrine\ORM\Tools\Debug`: `export()`, a copy of a value whose
//! objects are `stdClass` objects with a `__CLASS__` and their properties
//! down to a depth.
//!
//! `(array) $var` is the object's `properties()`: an entity object gives
//! its initialized properties under their plain names (it keeps no
//! visibility per slot), and an object whose `properties()` gives PHP's
//! mangled keys (`"\0*\0name"`, `"\0Class\0name"`) is exported with
//! `name:protected` and `name:Class:private`. `dump()` (`var_dump()` of
//! the export, through the output buffer) is not ported; its one caller
//! is the `orm:run-dql` command.
//!
//! Upstream: doctrine/orm src/Tools/Debug.php

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use php_value::datetime::{DateTime, DateTimeImmutable};
use php_value::object::{ArrayIterator, ArrayObject, StdClass};
use php_value::{Key, PhpArray, PhpObject, Thrown, Value};

use crate::collection_object::{self, CollectionRef, downcast};
use crate::object::EntityObject;

/// `Doctrine\Persistence\Proxy`.
const PROXY: &str = "Doctrine\\Persistence\\Proxy";

/// `Proxy::MARKER`.
const MARKER: &str = "__CG__";

/// `DefaultProxyClassNameResolver::getClass($object)`: the class after
/// the proxy marker.
fn get_class(object: &dyn PhpObject) -> String {
    let class = object.class();
    let marker = format!("\\{MARKER}\\");
    match class.rfind(&marker) {
        Some(position) => class[position + marker.len()..].to_owned(),
        None => class.to_owned(),
    }
}

/// `count($var)` as PHP's `count()` gives it for an array.
fn count(array: &PhpArray) -> i64 {
    i64::try_from(array.len()).unwrap_or(i64::MAX)
}

/// `Debug::export($var, $maxDepth)`.
pub fn export(
    var: &Value,
    max_depth: i64,
) -> Pin<Box<dyn Future<Output = Result<Value, Thrown>> + Send + '_>> {
    Box::pin(async move {
        let var = if CollectionRef::of(var).is_some() {
            Value::Array(collection_object::to_array(var).await?)
        } else {
            var.clone()
        };

        if max_depth == 0 {
            return Ok(match &var {
                Value::Object(object) => Value::string(object.class()),
                Value::Array(array) => Value::string(format!("Array({})", count(array))),
                other => other.clone(),
            });
        }

        let object = match &var {
            Value::Array(array) => {
                let mut r#return = PhpArray::new();

                for (k, v) in array.iter() {
                    r#return.insert(k.clone(), export(v, max_depth - 1).await?);
                }

                return Ok(Value::Array(r#return));
            }
            Value::Object(object) => object.clone(),
            other => return Ok(other.clone()),
        };

        let mut r#return = PhpArray::new();
        let date = downcast::<DateTimeImmutable>(&object)
            .map(|date| *date)
            .or_else(|| downcast::<DateTime>(&object).map(|date| date.get()));
        if let Some(date) = date {
            r#return.insert("__CLASS__", Value::string(object.class()));
            r#return.insert("date", Value::string(date.format("c")));
            r#return.insert("timezone", Value::string(date.timezone().name()));

            return Ok(StdClass::object(r#return));
        }

        r#return.insert("__CLASS__", Value::string(get_class(object.as_ref())));

        if object.instance_of(PROXY) {
            let initialized = downcast::<EntityObject>(&object)
                .and_then(|entity| entity.proxy_state())
                .is_some_and(|(initialized, _)| initialized);
            r#return.insert("__IS_PROXY__", Value::Bool(true));
            r#return.insert("__PROXY_INITIALIZED__", Value::Bool(initialized));
        }

        let storage = downcast::<ArrayObject>(&object)
            .map(|array| array.storage().clone())
            .or_else(|| downcast::<ArrayIterator>(&object).map(|it| it.storage().clone()));
        if let Some(storage) = storage {
            r#return.insert(
                "__STORAGE__",
                export(&Value::Array(storage), max_depth - 1).await?,
            );
        }

        fill_return_with_class_attributes(&object, r#return, max_depth).await
    })
}

/// Fill the `$return` variable with class attributes. Based on obj2array
/// function from <https://secure.php.net/manual/en/function.get-object-vars.php#47075>.
async fn fill_return_with_class_attributes(
    var: &Arc<dyn PhpObject>,
    mut r#return: PhpArray,
    max_depth: i64,
) -> Result<Value, Thrown> {
    let clone = var.properties();

    for (key, value) in clone.iter() {
        let key_text = match key {
            Key::Int(i) => i.to_string().into_bytes(),
            Key::Str(s) => s.as_bytes().to_vec(),
        };
        let aux: Vec<&[u8]> = key_text.split(|&b| b == 0).collect();
        let mut name = aux.last().copied().unwrap_or_default().to_vec();
        if aux[0].is_empty() && aux.len() > 1 {
            name.push(b':');
            if aux[1] == b"*" {
                name.extend_from_slice(b"protected");
            } else {
                name.extend_from_slice(aux[1]);
                name.extend_from_slice(b":private");
            }
        }

        r#return.insert(Key::from_bytes(&name), export(value, max_depth - 1).await?);
    }

    Ok(StdClass::object(r#return))
}
