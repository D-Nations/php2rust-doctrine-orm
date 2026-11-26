//! The test assets of tests/DoctrineTest/InstantiatorTestAsset, declared
//! in the class registry as PHP declares them (abstract, final, parent,
//! interfaces), and stand-ins for the PHP classes they extend (internal
//! classes, here declared by the test with the flags PHP gives them).
//!
//! Upstream: doctrine/instantiator tests/DoctrineTest/InstantiatorTestAsset/AbstractClassAsset.php tests/DoctrineTest/InstantiatorTestAsset/ArrayObjectAsset.php tests/DoctrineTest/InstantiatorTestAsset/ExceptionAsset.php tests/DoctrineTest/InstantiatorTestAsset/FinalExceptionAsset.php tests/DoctrineTest/InstantiatorTestAsset/PharExceptionAsset.php tests/DoctrineTest/InstantiatorTestAsset/SerializableArrayObjectAsset.php tests/DoctrineTest/InstantiatorTestAsset/SerializableFinalInternalChildAsset.php tests/DoctrineTest/InstantiatorTestAsset/SimpleEnumAsset.php tests/DoctrineTest/InstantiatorTestAsset/SimpleSerializableAsset.php tests/DoctrineTest/InstantiatorTestAsset/SimpleTraitAsset.php tests/DoctrineTest/InstantiatorTestAsset/UnCloneableAsset.php tests/DoctrineTest/InstantiatorTestAsset/UnserializeExceptionArrayObjectAsset.php tests/DoctrineTest/InstantiatorTestAsset/WakeUpNoticesAsset.php tests/DoctrineTest/InstantiatorTestAsset/XMLReaderAsset.php

#![allow(dead_code, reason = "each test file uses part of the assets")]

use std::sync::{Arc, Once};

use php_value::object::{ArrayIterator, ArrayObject, EnumCase};
use php_value::{Object, PhpArray, Thrown};
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, MethodInfo, register_class};

pub const ASSET: &str = "DoctrineTest\\InstantiatorTestAsset\\";

pub fn asset(name: &str) -> String {
    format!("{ASSET}{name}")
}

/// `uniqid('', true)` without the dot: a name no class has.
pub fn uniqid() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_micros());
    format!("{now:x}{}", COUNTER.fetch_add(1, Ordering::SeqCst))
}

fn constructor() -> MethodInfo {
    MethodInfo::new("__construct")
}

pub fn declare_assets() {
    static DECLARED: Once = Once::new();
    DECLARED.call_once(|| {
        let array_object = || -> Result<Object, Thrown> { Ok(Arc::new(ArrayObject::new(PhpArray::new()))) };
        for info in [
            // PHP's classes.
            ClassInfo::interface("Traversable").internal(),
            ClassInfo::interface("IteratorAggregate")
                .internal()
                .implements("Traversable"),
            ClassInfo::interface("Iterator").internal().implements("Traversable"),
            ClassInfo::interface("ArrayAccess").internal(),
            ClassInfo::interface("Countable").internal(),
            ClassInfo::interface("Serializable").internal(),
            ClassInfo::interface("SeekableIterator")
                .internal()
                .implements("Iterator"),
            ClassInfo::class("ArrayObject")
                .internal()
                .implements("IteratorAggregate")
                .implements("ArrayAccess")
                .implements("Serializable")
                .implements("Countable")
                .method(constructor())
                .new_instance(Arc::new(array_object)),
            ClassInfo::class("ArrayIterator")
                .internal()
                .implements("SeekableIterator")
                .implements("ArrayAccess")
                .implements("Serializable")
                .implements("Countable")
                .method(constructor())
                .new_instance(Arc::new(|| {
                    Ok(Arc::new(ArrayIterator::new(PhpArray::new())) as Object)
                })),
            ClassInfo::class("XMLReader").internal(),
            ClassInfo::class("PharException").internal().extends("Exception"),
            ClassInfo::class("PDORow").internal().final_class().new_instance(Arc::new(|| {
                Err(Thrown::new(
                    "Exception",
                    "Unserialization of 'PDORow' is not allowed",
                    0,
                ))
            })),
            // The assets.
            ClassInfo::class(asset("AbstractClassAsset")).abstract_class(),
            ClassInfo::class(asset("ArrayObjectAsset"))
                .extends("ArrayObject")
                .method(constructor()),
            ClassInfo::class(asset("ExceptionAsset"))
                .extends("Exception")
                .method(constructor()),
            ClassInfo::class(asset("FinalExceptionAsset"))
                .final_class()
                .extends("Exception")
                .method(constructor()),
            ClassInfo::class(asset("PharExceptionAsset"))
                .extends("PharException")
                .method(constructor()),
            ClassInfo::class(asset("SerializableArrayObjectAsset"))
                .extends("ArrayObject")
                .implements("Serializable")
                .method(constructor())
                .method(MethodInfo::new("unserialize"))
                .method(MethodInfo::new("__unserialize")),
            ClassInfo::class(asset("SerializableFinalInternalChildAsset"))
                .final_class()
                .extends("ArrayIterator"),
            ClassInfo::enumeration(
                asset("SimpleEnumAsset"),
                vec![
                    Arc::new(EnumCase::new(asset("SimpleEnumAsset"), "Foo", None)),
                    Arc::new(EnumCase::new(asset("SimpleEnumAsset"), "Bar", None)),
                ],
            ),
            ClassInfo::class(asset("SimpleSerializableAsset"))
                .implements("Serializable")
                .method(constructor())
                .method(MethodInfo::new("serialize"))
                .method(MethodInfo::new("unserialize"))
                .method(MethodInfo::new("__serialize"))
                .method(MethodInfo::new("__unserialize")),
            ClassInfo::trait_(asset("SimpleTraitAsset")),
            ClassInfo::class(asset("UnCloneableAsset"))
                .method(constructor())
                .method(MethodInfo::new("__clone")),
            ClassInfo::class(asset("UnserializeExceptionArrayObjectAsset"))
                .extends("ArrayObject")
                .method(MethodInfo::new("__wakeup")),
            ClassInfo::class(asset("WakeUpNoticesAsset"))
                .extends("ArrayObject")
                .method(MethodInfo::new("__wakeup")),
            ClassInfo::class(asset("XMLReaderAsset"))
                .extends("XMLReader")
                .method(constructor()),
        ] {
            register_class(info);
        }
    });
}
