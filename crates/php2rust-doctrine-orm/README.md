# Doctrine ORM

Private workspace crate: a port of `doctrine/orm` 3.7.5 (`00c2fd70`), in
progress. MIT; Doctrine source and tests copyright the Doctrine Project.
Unofficial, not affiliated with or endorsed by Doctrine.

```rust
use php2rust_doctrine_lexer::AbstractLexer;
use php2rust_doctrine_orm::query::{Lexer, TokenType};
use php_value::PhpString;

let mut lexer = Lexer::new("SELECT u FROM App\\User u WHERE u.name = 'O''Neil'").unwrap();
let mut types = Vec::new();
while lexer.move_next() {
    types.push(lexer.lookahead().unwrap().type_.unwrap());
}
assert_eq!(TokenType::T_SELECT, types[0]);
assert_eq!(TokenType::T_FULLY_QUALIFIED_NAME, types[3]);
assert_eq!(TokenType::T_STRING, types[10]);
assert_eq!(PhpString::from("O'Neil"), lexer.token().unwrap().value);
```

| Contract | Source | Evidence |
|---|---|---|
| `Query\TokenType`: every case with its value and name, `cases()`, `tryFrom()`, the case of a name | `Query/TokenType.php` | Query LexerTest; recorded tokens |
| `Query\Lexer`: the DQL patterns on doctrine/lexer; `getType()`: numbers (`T_FLOAT` with `.` or `e`), quoted strings unquoted (`''` to `'`), keywords and the names `T_<NAME>` of cases above 100 in any ASCII case, fully qualified names, input parameters, symbols, `T_NONE`; values stay strings | `Query/Lexer.php` | Query LexerTest (15); 1,201 inputs recorded from PHP 8.4.27 (`tests/data/query_lexer.json`, oracle/recorders/orm_query_lexer.php, `tests/query_lexer_vectors.rs`) |
| `Query\Expr`: every builder method and the string each expression gives; `in()`/`notIn()` quote each element of an array or `\Traversable` that is not a `Literal`; `literal()` quotes strings (`'` doubled), writes bools as `true`/`false` and numbers as PHP converts them; `andX()`/`orX()` take `Comparison`, `Func`, `Andx`, `Orx` and strings, and convert scalars and other `\Stringable` objects as PHP does for a caller without `strict_types` (`TypeError` for `null`, arrays and other objects) | `Query/Expr.php` | ExprTest (56); 1,463 expressions recorded from PHP 8.4.27 (`tests/data/query_expr.json`, oracle/recorders/orm_query_expr.php, `tests/query_expr_vectors.rs`) |
| `Expr\Base` and its subclasses (`Composite`, `Andx`, `Orx`, `Select`, `GroupBy`, `Literal`): constructor (an array whose element 0 is an array stands for it), `add()` (skips `null` and empty expressions; `InvalidArgumentException` "Expression of type '%s' not allowed in this context." for a value that is not a string or an object of an allowed class), `addMultiple()` over `(array) $args` (with an expression's protected properties), `count()`, `getParts()`; `Composite` puts a nested composite of several parts, or a part with ` OR `/` AND ` between white space, in parentheses | `Query/Expr/Base.php`, `Composite.php`, `Andx.php`, `Orx.php`, `Select.php`, `GroupBy.php`, `Literal.php` | ExprTest; recorded expressions |
| `Expr\Comparison`, `Func`, `Math` (a `Math` operand in parentheses), `From`, `Join` (alias, index and condition only when truthy; `strtoupper()`'s `TypeError` for a condition without a type), `OrderBy` (`SortDirection` to `ASC`/`DESC`, a falsy order to `ASC`, the deprecation `https://github.com/doctrine/orm/issues/11313` for an order that is not a `SortDirection`, in the constructor and again in `add()`) | `Query/Expr/Comparison.php`, `Func.php`, `Math.php`, `From.php`, `Join.php`, `OrderBy.php` | ExprTest; recorded expressions (with the deprecation counts) |
| `ParameterTypeInferer::inferType()`: DBAL's `Types` names for ints, bools, dates, intervals and backed enums, `ArrayParameterType` from an array's first element (an enum's backing value), `ParameterType::STRING` otherwise; types from php2rust-doctrine-dbal | `Query/ParameterTypeInferer.php` | ParameterTypeInfererTest (1, 15 rows) |
| `Parameter`: the name without leading and trailing `:`, the value, a given type or the inferred one (`$type ?:`: also for a falsy type name), `typeWasSpecified()` | `Query/Parameter.php` | Compiled use; no upstream test of its own |
| `Mapping\ClassMetadata`: the mapping API (`mapField()`, the association `map*()` methods, overrides, inheritance, discriminator, lifecycle callbacks, entity listeners, id generators, sequences, `__sleep()`/`unserialize()`, `newInstance()`), with the `MappingException` texts; the metadata is shared as `ClassMetadata` (an `RwLock` over `ClassMetadataInfo`) | `Mapping/ClassMetadata.php`, `MappingException.php`, `Exception/*.php` | ClassMetadataTest (69) |
| The mapping value objects: `FieldMapping`, `AssociationMapping` and its subclasses, `JoinColumnMapping`, `JoinTableMapping`, `EmbeddedClassMapping`, `DiscriminatorColumnMapping` (`fromMappingArray()`, `toArray()`, `ArrayAccess`, `__sleep()`) | `Mapping/*Mapping.php` | the `*MappingTest` files and TableMappingTest (23) |
| The mapping attributes (`Entity`, `Table`, `Column`, `Id`, `GeneratedValue`, the association and join attributes, `OrderBy`, overrides, lifecycle callbacks, ...) with PHP's constructor defaults; `AttributeReader` and `AttributeDriver` over a registry that stands in for PHP attributes (`ClassAttributes`, `register_attributes()`: what a derive macro would emit) | `Mapping/*.php` (attributes), `Driver/AttributeReader.php`, `Driver/AttributeDriver.php` | AttributeReaderTest (5), AttributeDriverTest with MappingDriverTestCase (62); test models recorded from PHP 8.3.6 reflection (`tests/data/models.json`, oracle/recorders/orm_models.php) |
| `ClassMetadataFactory` on persistence's `AbstractClassMetadataFactory`: inheritance, embeddables, id generator completion (identity preferences, DBAL 4 PostgreSQL identity, Oracle sequences), default discriminator map, runtime validation | `Mapping/ClassMetadataFactory.php` | ClassMetadataFactoryTest (12), BasicInheritanceMappingTest (14), ClassMetadataLoadEventTest (1) |
| The builders (`ClassMetadataBuilder`, `FieldBuilder`, `EmbeddedBuilder`, the association builders, `EntityListenerBuilder`) | `Mapping/Builder/*.php` | ClassMetadataBuilderTest (38), FieldBuilderTest (1) |
| Naming strategies (`DefaultNamingStrategy`, `UnderscoreNamingStrategy`), quote strategies (`DefaultQuoteStrategy`, `AnsiQuoteStrategy`), typed field mappers (`DefaultTypedFieldMapper`, `ChainTypedFieldMapper`) | `Mapping/*NamingStrategy.php`, `*QuoteStrategy.php`, `*TypedFieldMapper.php`, `Internal/SQLResultCasing.php` | NamingStrategyTest (6), QuoteStrategyTest (8), AnsiQuoteStrategyTest (8), DefaultQuoteStrategyTest (2), TypedFieldMapperTest (1, 12 rows), TypedEnumFieldMapperTest (1) |
| The property accessors (`RawValue`, `ObjectCast`, `TypedNoDefault`, `Readonly`, `Enum`, `Embeddable`, the factory) and the legacy `ReflectionReadonlyProperty`/`ReflectionEmbeddedProperty` | `Mapping/PropertyAccessors/*.php`, `ReflectionReadonlyProperty.php`, `ReflectionEmbeddedProperty.php` | the PropertyAccessors tests (10), ReflectionReadonlyPropertyTest (3), ReflectionEmbeddedPropertyTest (2, 6 rows each) |

`Lexer::new()` returns doctrine/lexer's `PcreError`, which the DQL
patterns never cause.

Query expressions, not reproduced and so not claimed:

- PHP's `mixed` is `php_value::Value`; an expression is passed with
  `Value::from(expression)`. `__toString()` is `try_to_string()`, which
  returns PHP's `Error` for a part without a string form; converting an
  array gives `"Array"` (PHP also warns).
- String parameters (`string $x`, operators, join and from names) are
  `&str`; the `TypeError`s PHP gives for other values there cannot
  happen. `between()` converts its bounds as strings, without the
  `int|string` check. The variadic methods take a `Vec`, so
  `NoUnknownNamedArguments` has nothing to check.
- The `allowedClasses` checks compare the exact class, as upstream's
  `in_array(..., true)` does; Rust has no subclasses of these classes.
- PHP's exception messages end with ", called in ... on line N" for a
  parameter's `TypeError`; these do not.
- `crate::deprecations` (shared by Query and Mapping) counts per thread (PHP has one process-wide
  state); `SortDirection` is symfony/polyfill-php86's enum.

Pending: `ExprTest::testExistsExpr`, `testAllExpr`, `testSomeExpr`,
`testAnyExpr` and `testNotExpr` build a `QueryBuilder` from the test
entity manager, which are not ported.

Mapping, not reproduced and so not claimed:

- PHP attributes are a registry (`mapping::attributes`); the test models
  are registered from PHP reflection recorded by
  oracle/recorders/orm_models.php (locally, PHP 8.3.6, not the Docker
  oracle). There are no recorded ORM vectors beyond the models.
- There is no entity manager: `ClassMetadataFactory::set_entity_manager()`
  takes a `FactoryConfiguration` (driver, naming and quote strategies,
  typed field mapper, identity preferences, platform), and the
  `loadClassMetadata` and `onClassMetadataNotFound` listeners are hooks
  on it. Only DBAL's `SQLitePlatform` exists; other platforms are known
  by class name.
- No dynamic properties; `LegacyReflectionFields` and
  `getMetadataValue()` are not ported. Test objects are `EntityObject`s,
  whose private properties of a parent share the child's slots.
- `IdGenerator` names the generator class and its settings; generating
  values needs the entity manager.
- Unserialized metadata gets the default naming strategy and typed field
  mapper (PHP leaves them uninitialized).
- `getSequenceName()`/`getSequencePrefix()` take no platform (upstream
  ignores it). A `ReflectionException` from the driver becomes
  `MappingException::reflectionFailure()` without the previous
  exception (`Thrown` has none).
- The attribute constructors' type checks (such as an invalid override
  type) cannot fail: the Rust types rule those values out.
- PHPUnit's `#[Depends]` chains rebuild the metadata in each test.

Mapping pending: XmlMappingDriverTest, the Symfony XmlDriverTest,
StaticPHPMappingDriverTest and `ClassMetadataTest::testClassNameMappingDiscriminatorValue`
(the XML and static PHP drivers are not ported; the demo uses
attributes); `RawValuePropertyAccessorTest` (PHP 8.4 property hooks);
`EntityListenerResolverTest` (the resolver serves the unit of work);
`AttributeDriverTest::testMappedSuperclassWithRepository`,
`ClassMetadataFactoryTest::testAcceptsEntityManagerInterfaceInstances`
and `QuoteStrategyTest::testConfiguration` (the entity manager and its
`Configuration`).
