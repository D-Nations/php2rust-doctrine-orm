# Doctrine Instantiator

Private workspace crate: a port of `doctrine/instantiator` 2.1.0
(`23da848e`). MIT; Doctrine source and tests copyright the Doctrine
Project. Unofficial, not affiliated with or endorsed by Doctrine.

Classes are the ones declared in persistence's class registry
(`php2rust_doctrine_persistence::php_reflection`); a class says how its
blank instance is made with `ClassInfo::new_instance()`, or gets a
`GenericObject` with its public defaults.

```rust
use php2rust_doctrine_instantiator::exception::ExceptionInterface;
use php2rust_doctrine_instantiator::{Instantiator, InstantiatorInterface};
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, PropertyInfo, register_class};
use php_value::{Key, Value};

register_class(
    ClassInfo::class("App\\Entity\\User")
        .property(PropertyInfo::new("name").default_value(Value::from("anonymous"))),
);
register_class(ClassInfo::class("App\\Entity\\Base").abstract_class());

let instantiator = Instantiator::new();
let user = instantiator.instantiate("App\\Entity\\User").unwrap();
assert_eq!("App\\Entity\\User", user.class());
assert_eq!(Some(&Value::from("anonymous")), user.properties().get(&Key::from("name")));

let error = instantiator.instantiate("App\\Entity\\Base").unwrap_err();
assert_eq!(
    "The provided class \"App\\Entity\\Base\" is abstract, and cannot be instantiated",
    error.as_exception_interface().unwrap().get_message()
);
```

| Contract | Source | Evidence |
|---|---|---|
| `InvalidArgumentException`: the interface, trait, unknown-class, abstract and enum messages, as upstream words them | `Exception/InvalidArgumentException.php` | InvalidArgumentExceptionTest (4); InstantiatorTest::testInstantiationFromNonExistingClass |
| `UnexpectedValueException`: the un-serialization messages, code 0, the previous exception (`new Exception($errorString, $errorCode)` for an unclean un-serialization) | `Exception/UnexpectedValueException.php` | UnexpectedValueExceptionTest (2) |
| `instantiate()`: the checks in upstream's order (not a class, an enum, abstract); `newInstanceWithoutConstructor()` unless the class is final with an internal ancestor; then the un-serialization path, tried once, where an `Exception` becomes `UnexpectedValueException::fromSerializationTriggeredException()` and an `Error` is returned as thrown; the factory is cached per class name, so later calls make a new instance each time | `Instantiator.php` | InstantiatorTest (5) |
| `ExceptionInterface`, `InstantiatorInterface` as traits; the instantiator's classes are declared in the registry (`declare_classes()`) | `Exception/ExceptionInterface.php`, `InstantiatorInterface.php` | Compiled use; the interface rows above |

Not reproduced, and so not claimed:

- PHP classes are known only once declared in the registry, with their
  flags (`internal()`, `final_class()`, `abstract_class()`). The tests
  declare the internal classes they use (`ArrayObject`, `PharException`,
  `XMLReader`, `PDORow`, ...) as PHP 8.4 declares them; their objects
  are stand-ins, not PHP's.
- `unserialize()` is not run: on that path a blank instance is the
  class's `new_instance()` factory, which is where `__wakeup()`,
  `__unserialize()` or `Serializable::unserialize()` belong. The `C:` and
  `O:` payload formats are not built.
- A warning or notice raised while un-serializing
  (`fromUncleanUnSerialization()` through `set_error_handler()`) cannot
  happen: a factory makes an instance or throws.
- Upstream clones a cached prototype for classes without `__clone()`;
  here the factory is called each time. Objects are shared handles in
  `php_value`, and a blank instance equals a clone of one.
