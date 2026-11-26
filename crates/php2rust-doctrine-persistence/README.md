# Doctrine Persistence

Private workspace crate: a port of `doctrine/persistence` 4.2.1
(`d8293831`). MIT; Doctrine source, tests and the test fixtures under
`tests/data/Mapping/` copyright the Doctrine Project. Unofficial, not
affiliated with or endorsed by Doctrine.

```rust
use php2rust_doctrine_persistence::mapping::driver::{MappingDriver, MappingDriverChain};
use php2rust_doctrine_persistence::mapping::{ReflectionService, RuntimeReflectionService};
use php2rust_doctrine_persistence::php_reflection::{ClassInfo, register_class};

register_class(ClassInfo::class("App\\Entity\\Base"));
register_class(ClassInfo::class("App\\Entity\\User").extends("App\\Entity\\Base"));

let reflection = RuntimeReflectionService::new();
assert_eq!(
    reflection.get_parent_classes("App\\Entity\\User").unwrap(),
    ["App\\Entity\\Base"]
);
assert!(MappingDriverChain::new().is_transient("App\\Entity\\User").unwrap());
```

PHP interfaces are traits over shared objects (`Arc<dyn Trait>` with
`&self` methods); abstract classes are traits that hold a state struct,
whose provided methods are the PHP methods (overridable; `parent::` calls
go to the free functions of the same module). What PHP throws is a
`php_value::Thrown` with PHP's class, message and code. The classes PHP's
reflection would find are declared with `php_reflection::register_class`.
The registry also carries what doctrine/instantiator reads: abstract,
final and internal flags, `trait_exists()`, `enum_exists()` and
`ReflectionClass::newInstanceWithoutConstructor()` (PHP's `Error` and
`ReflectionException` texts, checked against PHP 8.4.27; a blank instance
comes from the class's `new_instance()` factory, or is a `GenericObject`
with the public defaults).

| Contract | Source | Evidence |
|---|---|---|
| `ObjectManager`, `ObjectRepository`, `ConnectionRegistry`, `ManagerRegistry`, `NotifyPropertyChanged`, `PropertyChangedListener`, `Proxy` (with `MARKER`) as traits | `ObjectManager.php`, `ObjectRepository.php`, `ConnectionRegistry.php`, `ManagerRegistry.php`, `NotifyPropertyChanged.php`, `PropertyChangedListener.php`, `Proxy.php` | Compiled use; the tests below |
| `ObjectManagerDecorator` delegates every method to the wrapped manager | `ObjectManagerDecorator.php` | ObjectManagerDecoratorTest (13) |
| `AbstractManagerRegistry`: connections and managers by name, the default ones, `getManagerForClass()` through the proxy interface's parent class, `getRepository()`, `resetManager()`, upstream's `InvalidArgumentException` texts; a service that is not an `ObjectManager` is PHP's `AssertionError` (code 1) | `AbstractManagerRegistry.php` | ManagerRegistryTest (10); recorded `AssertionError` (`tests/persistence_vectors.rs`) |
| Event arguments; `PreUpdateEventArgs`'s change set and its `InvalidArgumentException` | `Event/*.php` | PreUpdateEventArgsTest (4) |
| `AbstractClassMetadataFactory`: loading parents first, `loadMetadata` / `onNotFoundMetadata` / `wakeupReflection`, the PSR-6 cache with the cache salt and key, `getAllMetadata()`, `isTransient()`, proxy class names through the resolver, `MappingException` texts | `Mapping/AbstractClassMetadataFactory.php`, `Mapping/ClassMetadataFactory.php`, `Mapping/ClassMetadata.php`, `Mapping/MappingException.php`, `Mapping/ProxyClassNameResolver.php` | AbstractClassMetadataFactoryTest (6), ClassMetadataFactoryTest (10) |
| `RuntimeReflectionService` and `ReflectionService`: short names, namespaces, parents, accessible properties (`TypedNoDefaultReflectionProperty` for an uninitialized typed property), `hasPublicMethod()` | `Mapping/RuntimeReflectionService.php`, `Mapping/ReflectionService.php` | RuntimeReflectionServiceTest (13) |
| Reflection properties: `RuntimeReflectionProperty` (proxies kept uninitialized while writing), `TypedNoDefaultReflectionProperty`, `EnumReflectionProperty` (`from()` on single values, `array_map` over arrays, PHP's `TypeError` / `ValueError` texts) | `Reflection/*.php` | RuntimeReflectionPropertyTest (3), TypedNoDefaultReflectionPropertyTest (3), EnumReflectionPropertyTest (15); 70 recorded `setValue()` and 6 `getValue()` cases and PHP's reflection exception texts (`tests/data/persistence.json`, oracle/recorders/persistence.php) |
| `MappingDriverChain` with its default driver and namespaces | `Mapping/Driver/MappingDriverChain.php`, `Mapping/Driver/MappingDriver.php` | DriverChainTest (6) |
| `DefaultFileLocator` and `SymfonyFileLocator` (prefixes, separators, `getAllClassNames()`, `fileExists()`, `findMappingFile()` texts) | `Mapping/Driver/DefaultFileLocator.php`, `Mapping/Driver/SymfonyFileLocator.php`, `Mapping/Driver/FileLocator.php` | DefaultFileLocatorTest (8), SymfonyFileLocatorTest (13) |
| `FileDriver` with its class cache, global basename and locator; the implementation's `loadMappingFile()` | `Mapping/Driver/FileDriver.php` | FileDriverTest (11) |
| `ClassNames`, `FileClassLocator` (classes declared in the matched files), `ColocatedMappingDriver` (paths, exclusions, file extension, transient classes) | `Mapping/Driver/ClassLocator.php`, `Mapping/Driver/ClassNames.php`, `Mapping/Driver/FileClassLocator.php`, `Mapping/Driver/ColocatedMappingDriver.php` | ClassNamesTest (1), FileClassLocatorTest (7), ColocatedMappingDriverTest (6) |
| `StaticPHPDriver`: `loadMetadata()` calls the class's static `loadMetadata`, class names from paths or a class locator | `Mapping/Driver/StaticPHPDriver.php` | StaticPHPDriverTest (3) |

Not ported: `PHPDriver` (it includes PHP mapping files at runtime).

Pending upstream tests:

- PHPDriverTest (4): `PHPDriver` is not ported.
- FileClassLocatorTest::testCreateFromSymfonyFinder: needs symfony/finder.
- FileClassLocatorTest::testWithPharFiles: needs Phar archives.
- StaticPHPDriverTest::testPublicApi: it lists the class's public methods
  with `ReflectionClass::getMethods()`; a Rust type has no such list.

Not reproduced, and so not claimed:

- Reflection knows only registered classes (`php_reflection`), not the
  classes PHP would autoload; `FileClassLocator` finds a class by the file
  it is registered with, not by including the file. An object's
  properties are found by name only (`ReflectionObject`); writing to an
  object of an unregistered type is an `Error` whose text is the port's.
- PHP's `TypeError`s for user functions end with `, called in <file> on
  line <line>`; the port's stop before that tail.
- `EnumReflectionProperty` passes `null` in an array to `from()` as `0`, as
  PHP 8.4 does, without PHP's deprecation notice. Assigning an enum of
  another class is checked by the typed property in PHP, not by the port.
- `setMetadataFor()` through the erased `ClassMetadataFactory` with metadata
  of another type is a `TypeError` whose text is the port's.
- The PSR-6 cache (`cache::ArrayCachePool`) keeps shared values, not
  serialized copies.
- Paths are strings; `realpath()` follows the platform.
