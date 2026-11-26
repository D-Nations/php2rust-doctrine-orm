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
| The mapping attributes (`Entity`, `Table`, `Column`, `Id`, `GeneratedValue`, the association and join attributes, `OrderBy`, overrides, lifecycle callbacks, ...) with PHP's constructor defaults; `AttributeReader` and `AttributeDriver` over a registry that stands in for PHP attributes (`ClassAttributes`, `register_attributes()`: what a derive macro would emit) | `Mapping/*.php` (attributes), `Driver/AttributeReader.php`, `Driver/AttributeDriver.php` | AttributeReaderTest (5), AttributeDriverTest with MappingDriverTestCase (62); test models recorded from PHP 8.4.27 reflection (`tests/data/models.json`, oracle/recorders/orm_models.php) |
| `ClassMetadataFactory` on persistence's `AbstractClassMetadataFactory`: inheritance, embeddables, id generator completion (identity preferences, DBAL 4 PostgreSQL identity, Oracle sequences), default discriminator map, runtime validation | `Mapping/ClassMetadataFactory.php` | ClassMetadataFactoryTest (12), BasicInheritanceMappingTest (14), ClassMetadataLoadEventTest (1) |
| The builders (`ClassMetadataBuilder`, `FieldBuilder`, `EmbeddedBuilder`, the association builders, `EntityListenerBuilder`) | `Mapping/Builder/*.php` | ClassMetadataBuilderTest (38), FieldBuilderTest (1) |
| Naming strategies (`DefaultNamingStrategy`, `UnderscoreNamingStrategy`), quote strategies (`DefaultQuoteStrategy`, `AnsiQuoteStrategy`), typed field mappers (`DefaultTypedFieldMapper`, `ChainTypedFieldMapper`) | `Mapping/*NamingStrategy.php`, `*QuoteStrategy.php`, `*TypedFieldMapper.php`, `Internal/SQLResultCasing.php` | NamingStrategyTest (6), QuoteStrategyTest (8), AnsiQuoteStrategyTest (8), DefaultQuoteStrategyTest (2), TypedFieldMapperTest (1, 12 rows), TypedEnumFieldMapperTest (1) |
| The property accessors (`RawValue`, `ObjectCast`, `TypedNoDefault`, `Readonly`, `Enum`, `Embeddable`, the factory) and the legacy `ReflectionReadonlyProperty`/`ReflectionEmbeddedProperty` | `Mapping/PropertyAccessors/*.php`, `ReflectionReadonlyProperty.php`, `ReflectionEmbeddedProperty.php` | the PropertyAccessors tests (10), ReflectionReadonlyPropertyTest (3), ReflectionEmbeddedPropertyTest (2, 6 rows each) |
| `Query\Parser`: the DQL grammar (`QueryLanguage()` down to the terminals), the deferred validations (identification and result variables, partial objects, path expressions, `NEW` objects), the built-in and custom string, numeric and datetime functions, the `QueryException` syntax and semantical texts; `AbstractSchemaName()`, `match()` | `Query/Parser.php`, `Query/QueryException.php`, `Query/AST/ASTException.php` | LanguageRecognitionTest (112 of 113), ParserTest (6) |
| `Query\AST`: the nodes (as `ast::Node`, double-dispatched to the walker) and the functions (`ABS` ... `UPPER`, `IDENTITY`, `SIZE`, `DATE_ADD`/`DATE_SUB`, the bit functions); `ExpressionWithReturnType` and the deprecated `TypedExpression` (deprecation `https://github.com/doctrine/orm/pull/12543`) | `Query/AST/*.php`, `Query/AST/Functions/*.php` | the SqlGeneration tests; TypedExpressionDeprecationTest (2) |
| `Query\SqlWalker` and `SqlOutputWalker`: the SQL of SELECT, UPDATE and DELETE statements on the platform, table aliases, class table and single table inheritance, ordered collections, filters, lock hints, `HINT_FORCE_PARTIAL_LOAD`, limit and offset, custom DBAL types' `convertToPHPValueSQL()`, the result set mapping the walker fills; `OutputWalker::getFinalizer()` | `Query/SqlWalker.php`, `Query/SqlOutputWalker.php`, `Query/OutputWalker.php` | SelectSqlGenerationTest (172 of 185), UpdateSqlGenerationTest (18), DeleteSqlGenerationTest (28), SqlWalkerTest (2) |
| `TreeWalker`, `TreeWalkerAdapter`, `TreeWalkerChain`: walkers that change the AST (`&mut`) before the output walker; `setQueryComponent()`'s check | `Query/TreeWalker.php`, `TreeWalkerAdapter.php`, `TreeWalkerChain.php` | CustomTreeWalkersTest (5), CustomTreeWalkersJoinTest (2) |
| `ParserResult`, the executors and finalizers (`SqlExecutor` with its kind and SQL statements, `execute()` on the connection with bound parameters, the multi-table executors' temporary tables, `SingleSelectSqlFinalizer`, `PreparedExecutorFinalizer`), the deprecation `https://github.com/doctrine/orm/pull/11188` | `Query/ParserResult.php`, `Query/Exec/*.php` | ParserResultTest (5), Functional QueryTest |
| `Query` and `AbstractQuery`: `EntityManager::createQuery()`, DQL and state, parameters (`setParameter()` with name normalization, `setParameters()` from parameters or an array, `processParameterValue()` for entities, collections, arrays and backed enums), the parameter count checks, `setFirstResult()`/`setMaxResults()`, hints and default hints, lock modes (`TransactionRequiredException`), `setFetchMode()`, `getSQL()`, `execute()`, `getResult()`, `getArrayResult()`, `getScalarResult()`, `getSingleColumnResult()`, `getSingleResult()`, `getOneOrNullResult()`, `getSingleScalarResult()` (`NoResultException`, `NonUniqueResultException`), `toIterable()`, `free()`, `clone` | `Query.php`, `AbstractQuery.php`, `NoResultException.php`, `NonUniqueResultException.php`, `UnexpectedResultException.php` | Query QueryTest (32 of 39), Functional QueryTest (39 of 41) |
| The hydrators: `AbstractHydrator` (column information, `gatherRowData()`, `gatherScalarRowData()`, `buildEnum()`, `registerManaged()`), `ObjectHydrator` (fetch joins, collections, index by, `NEW` objects, mixed results, discriminators, proxies, `HydrationException`s), `ArrayHydrator`, `ScalarHydrator`, `SingleScalarHydrator`, `ScalarColumnHydrator`, `SimpleObjectHydrator` (discriminators, joined inheritance fields, enum checks with `MappingException::invalidEnumValue()`) | `Internal/Hydration/*.php`, `Exception/MultipleSelectorsFoundException.php` | ObjectHydratorTest (31), ArrayHydratorTest (19), ScalarHydratorTest (3), SingleScalarHydratorTest (4), ScalarColumnHydratorTest (4), SimpleObjectHydratorTest (6), AbstractHydratorTest (2 of 6), ResultSetMappingTest (3) |
| `NativeQuery` and `EntityManager::createNativeQuery()` | `NativeQuery.php` | Compiled use; no upstream test of its own |
| `TopologicalSort` and `StronglyConnectedComponents` (commit order, cycle breaking on optional edges, `CycleDetectedException`) | `Internal/TopologicalSort.php`, `Internal/StronglyConnectedComponents.php` | TopologicalSortTest, StronglyConnectedComponentsTest |
| `EntityManager` (async where it reaches the database): `create()` (`MissingMappingDriverImplementation`), `find()`/`find_with_lock()` (identifier normalization, `invalidCompositeIdentifier`, `MissingIdentifierField`, `UnrecognizedIdentifierFields`, lock requirements: `OptimisticLockException::notVersioned`, `TransactionRequiredException`), `getReference()`, `persist()`, `remove()`, `refresh()`, `detach()`, `contains()`, `clear()`, `close()` (`EntityManagerClosed`), `flush()`, `wrapInTransaction()`, `beginTransaction()`/`commit()`/`rollback()`, `getRepository()`, `lock()`; `Configuration` (`setDefaultRepositoryClassName()`'s `InvalidEntityRepository`) | `EntityManager.php`, `Configuration.php`, `Proxy/ProxyFactory.php` (native lazy ghosts) | BasicFunctionalTest (23), EntityRepositoryTest (35) |
| `UnitOfWork` (UnitOfWorkTest runs on a stubbed driver connection, as upstream's mocks): change sets (scalars, to-one, collections, `recomputeSingleEntityChangeSet()`), commit order by `TopologicalSort`, identity map (`EntityIdentityCollisionException`), cascades (persist, remove, refresh, detach), orphan removal, collection deletions and updates, entity states, `createEntity()` (lazy to-one proxies, deferred eager loads), the ORMInvalidArgumentException texts (`newEntityFoundThroughRelationship`, `invalidAssociation`, ...) | `UnitOfWork.php`, `ORMInvalidArgumentException.php`, `Exception/*.php`, `EntityNotFoundException.php`, `OptimisticLockException.php` | BasicFunctionalTest, UnitOfWorkTest (12), the OneToMany*/ManyToMany* association tests (37), OneToManyOrphanRemovalTest (5), LifecycleCallbackTest (7) |
| `PersistentCollection` over collections' `ArrayCollection` (lazy loading, snapshot, insert/delete diffs, dirty flag, orphan scheduling), and an `ArrayCollection` property object | `PersistentCollection.php` | the association tests, ManyToManyBasicAssociationTest (14) |
| `BasicEntityPersister` (inserts with identity ids, updates with versions, deletes with join table rows, `load()`, `loadAll()`, `loadById()`, `count()`, `exists()`, `refresh()`, the SELECT/INSERT/COUNT SQL, criteria with `IN` and `NULL`, `InvalidOrientation`, `UnrecognizedField`, `InvalidFindByCall`), `OneToManyPersister` and `ManyToManyPersister` (`delete()`, `update()`), `PersisterHelper`, `IdentifierFlattener`, the simple object hydrator | `Persisters/Entity/BasicEntityPersister.php`, `Persisters/Collection/*Persister.php`, `Utility/*.php`, `Internal/Hydration/SimpleObjectHydrator.php` | EntityRepositoryTest, BasicFunctionalTest, ManyToManyBasicAssociationTest (query counts) |
| `EntityRepository` (`find()`, `findAll()`, `findBy()`, `findOneBy()`, `count()`, `__call()` magic finders with `InvalidMagicMethodCall`), `DefaultRepositoryFactory` | `EntityRepository.php`, `Repository/*.php` | EntityRepositoryTest |
| Lifecycle events: `ListenersInvoker` (callbacks, entity listeners, event manager), the event args classes, `PreUpdateEventArgs` | `Event/*.php` | LifecycleCallbackTest |
| `SchemaTool::createSchema()`/`dropSchema()`/`getSchemaFromMetadata()` for no and single-table inheritance (columns, join columns and tables, indexes, unique constraints, foreign keys) | `Tools/SchemaTool.php` | the functional tests' schemas |
| `ResultSetMapping` | `Query/ResultSetMapping.php` | ResultSetMappingTest (3); the persister's mappings; the SQL walker's result mappings |
| `QueryBuilder`: the DQL parts (`add()` with its `$append` check and the deprecation `https://github.com/doctrine/orm/pull/12051` for numeric join keys, `getDQLPart(s)()`, `resetDQLPart(s)()`), `select()`/`addSelect()`/`distinct()`, `delete()`/`update()` (the alias check), `from()`/`indexBy()`, the joins (root aliases), `set()`, `where()`/`andWhere()`/`orWhere()`, `groupBy()`, `having()` and its variants, `orderBy()`/`addOrderBy()` (the deprecation `https://github.com/doctrine/orm/issues/11313` for an order that is not a `SortDirection`, `Invalid sort direction`), `addCriteria()`, the root aliases and entities, parameters (`setParameter()` updates a parameter of the same normalized name, `createNamedParameter()`), first and max results (no limit for DELETE and UPDATE), hints, the second-level cache options, `getDQL()` (cached until a part changes), `getQuery()`, `__clone()`; `QueryType`; `EntityManager::createQueryBuilder()`, `getExpressionBuilder()`, `EntityRepository::createQueryBuilder()` | `QueryBuilder.php`, `QueryType.php`, `EntityManager.php`, `EntityRepository.php` | QueryBuilderTest (108 of 109), ExprTest's subquery tests (5) |
| `QueryExpressionVisitor`: a `Criteria`'s comparisons and composites as `Expr` objects with their parameters (the alias prefix, `_`-joined parameter names made unique by the parameter count, `CONTAINS`/`STARTS_WITH`/`ENDS_WITH` as `LIKE`, `MEMBER OF`, `IS NULL` for a null value), the `RuntimeException` texts | `Query/QueryExpressionVisitor.php` | QueryExpressionVisitorTest (6) |
| `Tools\Pagination`: `CountWalker` (`HINT_DISTINCT`, the HAVING and two-FROM texts), `CountOutputWalker`, `LimitSubqueryWalker` (the identifier type hint, `HINT_ENABLE_DISTINCT`), `LimitSubqueryOutputWalker` (`SELECT DISTINCT <ids> FROM (...) dctrn_result`, the ORDER BY columns added as hidden selects, the inner ordered query; the `ROW_NUMBER()` SQL for PostgreSQL, SQL Server, Oracle and DB2), `WhereInWalker`, `RootTypeWalker`, `RowNumberOverFunction` (`RowNumberOverFunctionNotEnabled`), the RuntimeException texts; `Paginator`'s queries (`getCountQuery()`, `cloneQuery()`, `appendTreeWalker()`, `unbindUnusedQueryParams()`, `useOutputWalker()`) | `Tools/Pagination/{CountWalker,CountOutputWalker,LimitSubqueryWalker,LimitSubqueryOutputWalker,WhereInWalker,RootTypeWalker,RowNumberOverFunction,Paginator,PaginatorQuery}.php`, `Tools/Pagination/Exception/RowNumberOverFunctionNotEnabled.php` | CountWalkerTest (8), WhereInWalkerTest (1), LimitSubqueryWalkerTest (10), RootTypeWalkerTest (1), CountOutputWalkerTest (4 of 5), LimitSubqueryOutputWalkerTest (12 of 27) |

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

Query builder, not reproduced and so not claimed:

- The builder holds the entity manager as an `Arc<dyn QueryContext>`.
  `getQuery()` gives a `Query` of the builder's entity manager (through
  `QueryContext::as_entity_manager()`); a context that is not an
  `EntityManager` gives a `LogicException`. `expr()` and
  `getExpressionBuilder()` give a new `Expr` (it has no state).
- Parameters are values, not shared objects: `getQuery()` copies them,
  so a later `setParameter()` on the builder does not change the value
  in an earlier query (PHP clones the collection, not the parameters).
  `getParameters()` is a slice, `setParameters()` takes a `Vec`.
- A `QueryBuilder` is not a `php_value` object (those are `Send +
  Sync`); where PHP passes a builder to an expression (`exists($qb)`),
  `QueryBuilder::to_value()` gives a `Stringable` object holding the
  DQL at the time of the call. `clone` copies expression objects of the
  DQL parts; any other object in a part keeps its handle.
- `orderBy($sort)` without `$order` is `order_by(sort, None)`; an
  explicit `null` is `Some(Order::Null)`. Variadic arguments are `Vec`s;
  `where()` is `r#where()`. `getType()` (`final protected`) is public so
  that QueryBuilderTest's subclass test can call it.
- `QueryExpressionVisitor::walkComparison()` gives `isMemberOf()`'s
  `TypeError` (strict types) for a `MEMBER_OF` value that is not a
  string, without ", called in ...".

`QueryBuilderTest::testWhereWithUnexpectedNamedArguments` is n/a (Rust has
no named arguments).

Pagination, not reproduced and so not claimed:

- The paginator holds a `Query`, but `Paginator::count()` and
  `getIterator()` are not there yet, nor `getResultForOffset()` and
  `convertWhereInIdentifiersToDatabaseValues()`. The `PaginatorQuery`
  methods (private upstream) are public associated functions until then.
  PaginatorTest (3) and the functional PaginationTest are pending.
- `LimitSubqueryOutputWalker` borrows the query instead of cloning it:
  it reads the limit and offset when it is built and walks the inner
  query through a view with no limit and offset, so its `getQuery()` is
  the original query (`testSubqueryClonedCompletely` is pending). The
  ORDER BY column patterns are matched as text with ASCII case folding
  and the `[a-z0-9_]` boundaries of upstream's regex, without regex
  metacharacters in the alias or column. `platformSupportsRowNumber()`
  checks the platform classes; there is no `supportsRowNumberFunction()`.
  `clone $this` copies the walker's state and shares the parser result,
  as PHP does.
- `LimitSubqueryWalker::IDENTIFIER_TYPE`'s hint holds the DBAL type as
  `TypeObject`, a `php_value` object around the `Arc<dyn Type>`.
- Only `SQLitePlatform` exists: the walker tests on `MySQLPlatform` run on
  it (no `ROW_NUMBER()` and the same `LIMIT ... OFFSET ...`, so the SQL is
  upstream's); the PostgreSQL, Oracle and SQL Server cases are pending.
  `testParsingQueryWithDifferentLimitOffsetValuesTakesOnlyOneCacheEntry`
  is pending: there is no query cache.

Mapping, not reproduced and so not claimed:

- PHP attributes are a registry (`mapping::attributes`); the test models
  are registered from PHP reflection recorded by
  oracle/recorders/orm_models.php (PHP 8.4.27, the plain
  `php:8.4.27-cli-bookworm` image). There are no recorded ORM vectors beyond the models.
- `ClassMetadataFactory::set_entity_manager()` takes a
  `FactoryConfiguration` (the entity manager builds it from its
  `Configuration`) (driver, naming and quote strategies,
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

DQL, not reproduced and so not claimed:

- The parser and the walkers see the entity manager and the query
  through two traits, `query::context::QueryContext` (class metadata,
  the owning side of an association, the platform, `quote()`, the quote
  strategy, the custom functions, the enabled filters) and `DqlQuery`
  (the DQL, hints, hydration mode, parameters, first and max results).
  `EntityManager` implements `QueryContext` (its `quote()` is the
  driver's and needs an open connection; it has no filters yet); `Query`
  implements `DqlQuery`. The SQL generation tests implement `DqlQuery`
  in `tests/common/dql.rs` over `EntityManagerMock`'s stand-in, whose
  connection quotes as `'%s'`, as upstream's stub does.
- The `Configuration` names a custom function by class; Rust cannot
  instantiate a class by name, so a function class registers its
  constructor with `query::context::register_function_class()`. An
  unregistered class is an unknown function.
- The walker fills `query::ResultSetMapping` (`Query/ResultSetMapping.php`).
- Only DBAL's `SQLitePlatform` exists.
- Walker and tree walker classes are `OutputWalkerClass` and
  `TreeWalkerClass` values (a name and a constructor), not class-string
  hints. A walker method an AST node calls and PHP lacks
  (`walkWhenClauseExpression()`, ...) gives PHP's `Error` text.
- DQL is read as UTF-8 (lossy); assertions are treated as disabled, as
  in production PHP. `Thrown` has no previous exception, and a
  `TypeError` lacks ", called in ...".
- `query::utility` holds what the walkers use from the persisters and
  helpers outside `Query` (`PersisterHelper`,
  `HierarchyDiscriminatorResolver`, `LockSqlHelper`,
  `JoinedSubclassPersister::getOwningTable()`).
- Test parameters whose value is an entity or a `ClassMetadata` are
  `null`; the SQL does not depend on them. `setEntityNamespaces()` in
  `LanguageRecognitionTest::testRejectsInvalidDQL` is left out (the
  parser does not read it).

DQL pending: the SelectSqlGenerationTest tests on `MySQLPlatform`,
`PostgreSQLPlatform` and `OraclePlatform` (11; the DBAL port has only
`SQLitePlatform`).
Persistence, not reproduced and so not claimed:

- The DBAL is async, so the entity manager, unit of work, persisters,
  repositories and the collection methods that load (`count()`, `get()`,
  `contains()`, ...) are `async`; PHP's on-the-spot lazy loading is an
  explicit `initialize_object()`. A lazy proxy is an `EntityObject` ghost
  with its identifier set; `EntityManager::find()` initializes a proxy it
  finds in the identity map (PHP does on the first property access).
- The entity manager takes the `Configuration` by value: changes after
  `create()` are not seen. The platform is `SQLitePlatform`. A pessimistic
  lock runs no `SELECT 1 ... FOR UPDATE` query (SQLite has no lock
  clause), and `find()` with a pessimistic lock loads as `NONE` does.
- The persister does not join inverse one-to-one or EAGER to-one
  associations into its SELECT; `createEntity()` loads them with a second
  query. `matching()` (criteria on repositories and collections),
  filters, the second level cache, the metadata cache and the entity
  listener resolver are not ported; partial objects are not lazy
  ghosts of their selected fields. Joined (class table) inheritance persisters are `NotSupported`.
- Entity methods (lifecycle callbacks, entity listeners) are bodies
  registered by class and method name (`object::register_method()`,
  `register_listener_method()`), what a compiler would emit. A custom
  repository class is a name: the repository is an `EntityRepository`
  that reports it. `__call()` magic finders are `EntityRepository::call()`.
- `wrapInTransaction()` takes an async closure; `getReference()` of a
  class with subclasses is `NotSupported` (PHP calls `find()`).

Queries and hydration, not reproduced and so not claimed:

- The query keeps no parser result: it parses its DQL each time it needs
  the SQL or executes (the parser result is not `Send`; PHP keeps it and
  the query cache, which is not ported). There is no result cache,
  hydration cache or second level cache, so their setters and the
  cache tests are not ported.
- A result is read whole (the DBAL port fetches all rows): `toIterable()`
  returns the hydrated values in a `Vec` and the `onClear` listener of
  an iteration is not registered.
- Enum fields keep their backing values in the entity data; the simple
  object hydrator checks them and the enum property accessors build the
  cases. A `NEW` object's class registers its constructor with
  `register_new_object_class()` (Rust cannot instantiate a class by
  name); entities as `NEW` arguments are not supported. Custom hydration
  modes are not ported (`invalid_hydration_mode`), nor are `ClassMetadata`
  objects as parameter values.
- The query setters take `&self` and return `&Self` (PHP's fluent
  `static`); results are `Value`s.
- Reading a property of a lazy object does not load it: the tests call
  `initialize_object()` where PHP's property read would.

Persistence and query pending: `EntityRepositoryTest::testDefaultRepositoryClassName`
(the DDC753 models and changing the configuration after `create()`);
the `matching()` tests (EntityRepositoryTest `testMatching*`,
`testIsNullCriteria*`, `testFindByNullValueInMultipleInCriteriaValues`,
`testFindMultipleByNullValueInMultipleInCriteriaValues`, the
`ManyToManyBasicAssociationTest` and `OneToManyBidirectionalAssociationTest`
`testMatching*` and `testManyToManyOrderBy*` tests: criteria are not
ported); `EntityRepositoryTest::testCreateResultSetMappingBuilder`
(`ResultSetMappingBuilder` is not ported); the query and result cache
tests of Query QueryTest and `testProcessParameterValueClassMetadata`;
`Functional\QueryTest::testJoinPartialObjectHydration` (partial objects)
and `testQueryBuilderWithStringWhereClauseContainingOrAndConditionalPrimary`
(the query builder); `AbstractHydratorTest`'s `onClear` listener tests;
`CustomHydratorTest` (custom hydration modes); the remaining DQL-based
BasicFunctionalTest, LifecycleCallbackTest, ManyToManyEventTest and
`OneToManyUnidirectionalAssociationTest::testPersistOwningInverseCascade`
tests (not yet translated); the UnitOfWorkTest tests that read
`EntityPersisterMock`'s recorded inserts and its mocked id generator
type, and `UnitOfWorkTest::testItPreservesTheOriginalExceptionOnRollbackFailure`
(`Thrown` has no previous exception; a failing rollback's error replaces
the commit's).
