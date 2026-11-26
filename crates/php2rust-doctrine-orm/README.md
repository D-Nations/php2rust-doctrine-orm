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
| `Query` and `AbstractQuery`: `EntityManager::createQuery()`, DQL and state, parameters (`setParameter()` with name normalization, `setParameters()` from parameters or an array, `processParameterValue()` for entities, collections, arrays and backed enums), the parameter count checks, `setFirstResult()`/`setMaxResults()`, hints and default hints, lock modes (`TransactionRequiredException`), `setFetchMode()`, `getSQL()`, `execute()`, `getResult()`, `getArrayResult()`, `getScalarResult()`, `getSingleColumnResult()`, `getSingleResult()`, `getOneOrNullResult()`, `getSingleScalarResult()` (`NoResultException`, `NonUniqueResultException`), `toIterable()`, `free()`, `clone` | `Query.php`, `AbstractQuery.php`, `NoResultException.php`, `NonUniqueResultException.php`, `UnexpectedResultException.php` | Query QueryTest (32 of 39), Functional QueryTest (39 of 41), QueryIterableTest (4), QueryParameterTest (6), IndexByAssociationTest (5), TypeTest (9 of 11), TypeValueSqlTest (4), EnumTest (24), on SQLite |
| The hydrators: `AbstractHydrator` (column information, `gatherRowData()`, `gatherScalarRowData()`, `buildEnum()`, `registerManaged()`), `ObjectHydrator` (fetch joins, collections, index by, `NEW` objects with named arguments and entities as arguments, mixed results, discriminators, proxies, `HydrationException`s), `ArrayHydrator`, `ScalarHydrator`, `SingleScalarHydrator`, `ScalarColumnHydrator`, `SimpleObjectHydrator` (discriminators, joined inheritance fields, enum checks with `MappingException::invalidEnumValue()`) | `Internal/Hydration/*.php`, `Exception/MultipleSelectorsFoundException.php` | ObjectHydratorTest (31), ArrayHydratorTest (19), ScalarHydratorTest (3), SingleScalarHydratorTest (4), ScalarColumnHydratorTest (4), SimpleObjectHydratorTest (6), AbstractHydratorTest (2 of 6), ResultSetMappingTest (3), Functional/NewOperatorTest (38, on SQLite) |
| `Query\Filter`: `SQLFilter` (`setParameter()` with the inferred or given type, `setParameterList()`, `getParameter()` quoted by the connection, `getParameterList()` quoted and comma-joined, `hasParameter()`, `__toString()` as PHP's `serialize()` of the ksorted parameters, `getConnection()`, the `FilterException` texts), `FilterCollection` (`enable()`, `disable()`, `suspend()`, `restore()`, `getFilter()`, `has()`, `isEnabled()`, `isSuspended()`, `isClean()`, `getHash()`, `setFiltersStateDirty()`, the InvalidArgumentException texts); `Configuration::addFilter()`/`getFilterClassName()`; `EntityManager::getFilters()`, `isFiltersStateClean()`, `hasFilters()`; the filter conditions in the SQL walker and in the persister's `getSelectSQL()`, `getCountSQL()` and `exists()`; the many-to-many persister's `getFilterSql()` (the join of the target table `te` for the extra-lazy `count()`, `contains()`, `containsKey()` and `get()`) | `Query/Filter/SQLFilter.php`, `Query/Filter/Parameter.php`, `Query/Filter/FilterException.php`, `Query/FilterCollection.php`, `Configuration.php`, `EntityManager.php`, `Persisters/Entity/BasicEntityPersister.php`, `Persisters/Collection/ManyToManyPersister.php` | Functional/SQLFilterTest (32 of 49), FilterCollectionTest (9), ConfigurationTest::testAddGetFilters, the SwitchContextWithFilter ChangeFiltersTest (3) and the SwitchContextWithFilterAndIndexedRelation ChangeFiltersTest (1), on SQLite |
| `NativeQuery` and `EntityManager::createNativeQuery()` | `NativeQuery.php` | NativeQueryTest (20), Query NativeQueryTest (1) |
| `TopologicalSort` and `StronglyConnectedComponents` (commit order, cycle breaking on optional edges, `CycleDetectedException`) | `Internal/TopologicalSort.php`, `Internal/StronglyConnectedComponents.php` | TopologicalSortTest, StronglyConnectedComponentsTest |
| `EntityManager` (async where it reaches the database): `create()` (`MissingMappingDriverImplementation`), `find()`/`find_with_lock()` (identifier normalization, `invalidCompositeIdentifier`, `MissingIdentifierField`, `UnrecognizedIdentifierFields`, lock requirements: `OptimisticLockException::notVersioned`, `TransactionRequiredException`), `getReference()`, `persist()`, `remove()`, `refresh()`, `detach()`, `contains()`, `clear()`, `close()` (`EntityManagerClosed`), `flush()`, `wrapInTransaction()`, `beginTransaction()`/`commit()`/`rollback()`, `getRepository()`, `lock()`; `Configuration` (`setDefaultRepositoryClassName()`'s `InvalidEntityRepository`) | `EntityManager.php`, `Configuration.php`, `Proxy/ProxyFactory.php` (native lazy ghosts) | BasicFunctionalTest (33 of 34), EntityRepositoryTest (35), IdentityMapTest (5) |
| `UnitOfWork` (UnitOfWorkTest runs on a stubbed driver connection, as upstream's mocks): change sets (scalars, to-one, collections, `recomputeSingleEntityChangeSet()`), commit order by `TopologicalSort`, identity map (`EntityIdentityCollisionException`), cascades (persist, remove, refresh, detach), orphan removal, collection deletions and updates, entity states, `createEntity()` (lazy to-one proxies, deferred eager loads), the ORMInvalidArgumentException texts (`newEntityFoundThroughRelationship`, `invalidAssociation`, ...) | `UnitOfWork.php`, `ORMInvalidArgumentException.php`, `Exception/*.php`, `EntityNotFoundException.php`, `OptimisticLockException.php` | BasicFunctionalTest, UnitOfWorkTest (12), the OneToMany*/ManyToMany* association tests (37), OneToManyOrphanRemovalTest (5), LifecycleCallbackTest (7) |
| `PersistentCollection` over collections' `ArrayCollection` (lazy loading, snapshot, insert/delete diffs, dirty flag, orphan scheduling), and an `ArrayCollection` property object; `EXTRA_LAZY` collections (`count()`, `contains()`, `containsKey()` and `get()` of an `indexBy` collection, `slice()`, `first()` without loading the collection); `matching()` (in memory when initialized or dirty, else the one-to-many criteria on the back reference with the association's ordering, a `LazyCriteriaCollection` when extra lazy, or the many-to-many persister's criteria query) | `PersistentCollection.php` | the association tests, ManyToManyBasicAssociationTest (22), OneToManyBidirectionalAssociationTest (12), ExtraLazyCollectionTest (37), PersistentCollectionCriteriaTest (2), the ValueConversionType extra-lazy and criteria matching tests (14) |
| `BasicEntityPersister` (inserts with identity ids, updates with versions, deletes with join table rows, `load()`, `loadAll()`, `loadById()`, `count()`, `exists()` (with a `Criteria`), `refresh()`, the SELECT/INSERT/COUNT SQL, criteria with `IN` and `NULL`, `loadCriteria()`, `countCriteria()`, `expandCriteriaParameters()`, the comparison operators of a `Criteria` (`LIKE` for `CONTAINS`/`STARTS_WITH`/`ENDS_WITH`, `IS [NOT] NULL`, `IN`/`NOT IN` and `1=0` for an empty list), `InvalidOrientation`, `UnrecognizedField`, `InvalidFindByCall`, `MatchingAssociationFieldRequiresObject`), `SqlExpressionVisitor`, `SqlValueVisitor`, `OneToManyPersister` and `ManyToManyPersister` (`delete()`, `update()`, the extra-lazy `count()`, `slice()`, `containsKey()`, `contains()`, `get()`, and `loadCriteria()` of a many-to-many collection), `PersisterHelper`, `IdentifierFlattener`, the simple object hydrator; the inheritance persisters as kinds of `BasicEntityPersister` (`PersisterKind`): `AbstractEntityInheritancePersister` (the discriminator value in the insert data, the declaring class's column aliases), `SingleTablePersister` (the root table alias, the discriminator column and the subclasses' columns, the discriminator condition), `JoinedSubclassPersister` (inserts into the root table then the sub tables, updates and versions per owning table, deletes from the root table, the parent and subclass joins, the owning table of a field), `OneToManyPersister::deleteJoinedEntityCollection()` (the temporary id table) | `Persisters/Entity/BasicEntityPersister.php`, `Persisters/Entity/AbstractEntityInheritancePersister.php`, `Persisters/Entity/SingleTablePersister.php`, `Persisters/Entity/JoinedSubclassPersister.php`, `Persisters/SqlExpressionVisitor.php`, `Persisters/SqlValueVisitor.php`, `Persisters/MatchingAssociationFieldRequiresObject.php`, `Persisters/Collection/*Persister.php`, `Utility/*.php`, `Internal/Hydration/SimpleObjectHydrator.php` | EntityRepositoryTest, EntityRepositoryCriteriaTest (6), BasicFunctionalTest, ManyToManyBasicAssociationTest (query counts), ExtraLazyCollectionTest, SqlExpressionVisitorTest (1), SingleTableInheritanceTest (22), ClassTableInheritanceTest (16), ClassTableInheritanceSecondTest (2), MappedSuperclassTest (1), OneToOneSingleTableInheritanceTest (1), OrderedJoinedTableInheritanceCollectionTest (1), JoinedTableCompositeKeyTest (2), SingleTableCompositeKeyTest (2) |
| `EntityRepository` (`find()`, `findAll()`, `findBy()`, `findOneBy()`, `count()`, `__call()` magic finders with `InvalidMagicMethodCall`, `matching()`, `createResultSetMappingBuilder()`), `DefaultRepositoryFactory` | `EntityRepository.php`, `Repository/*.php` | EntityRepositoryTest (56) |
| `LazyCriteriaCollection`: the cached count, `isEmpty()` and `contains()` without loading, `matching()` in memory after one load | `LazyCriteriaCollection.php` | LazyCriteriaCollectionTest (7), EntityRepositoryCriteriaTest |
| Lifecycle events: `ListenersInvoker` (callbacks, entity listeners, event manager), the event args classes, `PreUpdateEventArgs` | `Event/*.php` | LifecycleCallbackTest |
| `SchemaTool::createSchema()`/`dropSchema()`/`getSchemaFromMetadata()` for no, single table and joined (class table) inheritance (columns, join columns and tables, indexes, unique constraints, foreign keys, the child tables' primary keys and `ON DELETE CASCADE` foreign keys to the root table) | `Tools/SchemaTool.php` | the functional tests' schemas, CompanySchemaTest (2) |
| `ResultSetMapping` | `Query/ResultSetMapping.php` | ResultSetMappingTest (3); the persister's mappings; the SQL walker's result mappings |
| `ResultSetMappingBuilder`: root and joined entities from class metadata (the column renaming modes, single table inheritance checks, column conflicts), `generateSelectClause()` with table aliases and custom types' `convertToPHPValueSQL()`, `__toString()` | `Query/ResultSetMappingBuilder.php` | NativeQueryTest, EntityRepositoryTest |
| `QueryBuilder`: the DQL parts (`add()` with its `$append` check and the deprecation `https://github.com/doctrine/orm/pull/12051` for numeric join keys, `getDQLPart(s)()`, `resetDQLPart(s)()`), `select()`/`addSelect()`/`distinct()`, `delete()`/`update()` (the alias check), `from()`/`indexBy()`, the joins (root aliases), `set()`, `where()`/`andWhere()`/`orWhere()`, `groupBy()`, `having()` and its variants, `orderBy()`/`addOrderBy()` (the deprecation `https://github.com/doctrine/orm/issues/11313` for an order that is not a `SortDirection`, `Invalid sort direction`), `addCriteria()`, the root aliases and entities, parameters (`setParameter()` updates a parameter of the same normalized name, `createNamedParameter()`), first and max results (no limit for DELETE and UPDATE), hints, the second-level cache options, `getDQL()` (cached until a part changes), `getQuery()`, `__clone()`; `QueryType`; `EntityManager::createQueryBuilder()`, `getExpressionBuilder()`, `EntityRepository::createQueryBuilder()` | `QueryBuilder.php`, `QueryType.php`, `EntityManager.php`, `EntityRepository.php` | QueryBuilderTest (108 of 109), ExprTest's subquery tests (5), Functional/QueryBuilderParenthesisTest (2, on SQLite) |
| `QueryExpressionVisitor`: a `Criteria`'s comparisons and composites as `Expr` objects with their parameters (the alias prefix, `_`-joined parameter names made unique by the parameter count, `CONTAINS`/`STARTS_WITH`/`ENDS_WITH` as `LIKE`, `MEMBER OF`, `IS NULL` for a null value), the `RuntimeException` texts | `Query/QueryExpressionVisitor.php` | QueryExpressionVisitorTest (6) |
| `Tools\Pagination`: `CountWalker` (`HINT_DISTINCT`, the HAVING and two-FROM texts), `CountOutputWalker`, `LimitSubqueryWalker` (the identifier type hint, `HINT_ENABLE_DISTINCT`), `LimitSubqueryOutputWalker` (`SELECT DISTINCT <ids> FROM (...) dctrn_result`, the ORDER BY columns added as hidden selects, the inner ordered query; the `ROW_NUMBER()` SQL for PostgreSQL, SQL Server, Oracle and DB2), `WhereInWalker`, `RootTypeWalker`, `RowNumberOverFunction` (`RowNumberOverFunctionNotEnabled`), the RuntimeException texts; `PaginatorQuery` (`getCountQuery()`, `cloneQuery()`, `appendTreeWalker()`, `unbindUnusedQueryParams()`, `useOutputWalker()`, `getResultForOffset()` with the fetch-join ID subquery + WHERE IN path and the output-walker path, `convertWhereInIdentifiersToDatabaseValues()`); `Paginator` (`count()`, `getIterator()`); `OffsetPaginator` and `PaginatorInterface` (`paginate()`, its InvalidArgumentException text); `Window`, `Page`, `WindowPage` (the LogicException texts) | `Tools/Pagination/{CountWalker,CountOutputWalker,LimitSubqueryWalker,LimitSubqueryOutputWalker,WhereInWalker,RootTypeWalker,RowNumberOverFunction,Paginator,PaginatorQuery,PaginatorInterface,OffsetPaginator,Page,Window,WindowPage}.php`, `Tools/Pagination/Exception/RowNumberOverFunctionNotEnabled.php` | CountWalkerTest (8), WhereInWalkerTest (1), LimitSubqueryWalkerTest (10), RootTypeWalkerTest (1), CountOutputWalkerTest (4 of 5), LimitSubqueryOutputWalkerTest (13 of 27), PaginatorTest (3), OffsetPaginatorTest (3), WindowTest (10 of 11), WindowPageTest (16), Functional/PaginationTest (56 of 57, on SQLite) |

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

- PHP's `PaginatorQuery` trait is a `PaginatorQuery` value that
  `Paginator` and `OffsetPaginator` hold; its methods (private upstream)
  are public, and `Paginator::paginator_query()` and `get_count_query()`
  reach them where the tests use reflection. `Query|QueryBuilder` is the
  `QueryOrBuilder` enum. `PaginatorInterface::paginate()` takes the
  position as a `&Value` (PHP's `mixed`): `Window` is a `php_value`
  object, so a non-window position gives upstream's
  InvalidArgumentException.
- `LimitSubqueryOutputWalker` keeps an `Rc<Query>` clone (parameters and
  hints copied, not cacheable, no limit or offset) for `getQuery()`
  (`get_clone_query()`), but walks the inner query through the borrowed
  query and a view of it with no limit and offset. The
  ORDER BY column patterns are matched as text with ASCII case folding
  and the `[a-z0-9_]` boundaries of upstream's regex, without regex
  metacharacters in the alias or column. `platformSupportsRowNumber()`
  checks the platform classes; there is no `supportsRowNumberFunction()`.
  `clone $this` copies the walker's state and shares the parser result,
  as PHP does.
- PaginatorTest and OffsetPaginatorTest replace the mocked connection and
  hydrator with a recording driver connection
  (`tests/common/recording_connection.rs`) that gives queued rows and
  records the bound values; the expected values are those bound after
  the array parameters are expanded (`[2, 1, 10]`).
- Functional/PaginationTest: `testDifferentResultLengthsDoNotRequireExtraQueryCacheEntries`
  is pending: there is no query cache. A data provider's rows run in one
  test. `CustomIdObject`'s `__toString()` is compared through its `id`.
  The models are recorded lines appended to `tests/data/models.json`.
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
  driver's and needs an open connection; its enabled filters are those
  of its `FilterCollection`); `Query`
  implements `DqlQuery`. The SQL generation tests implement `DqlQuery`
  in `tests/common/dql.rs` over `EntityManagerMock`'s stand-in, whose
  connection quotes as `'%s'`, as upstream's stub does.
- The `Configuration` names a custom function by class; Rust cannot
  instantiate a class by name, so a function class registers its
  constructor with `query::context::register_function_class()`. An
  unregistered class is an unknown function. A callable (PHP's
  `addCustom*Function()` with a `callable`) is
  `add_custom_*_function_callable()` with a `FunctionFactory` closure;
  adding a class of the same name replaces it. The functions are
  configured before `EntityManager::create()` (the configuration is
  taken by value), so CustomFunctionsTest (2) adds them there.
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
Filters, not reproduced and so not claimed:

- PHP's abstract `SQLFilter` is the `SqlFilter` struct; a subclass's
  `addFilterConstraint()` is a `FilterConstraint` object it holds. The
  configuration names a filter by class, so a filter class registers its
  constructor with `register_filter_class()`; an unregistered class gives
  PHP's `Class "X" not found` error.
- The entity manager creates its `FilterCollection` with itself (PHP does
  on the first `getFilters()`); a flag set by `getFilters()` stands for
  PHP's `filterCollection !== null` in `hasFilters()`.
- The parameters are kept sorted by byte order (PHP's `ksort()` gives the
  same order for the string names). `getConnection()` (protected) is
  public. Quoting needs an open connection.
- The persister applies the filters in `getSelectSQL()`, `getCountSQL()`
  and `exists()`; it joins no EAGER associations (they load with a second
  query, which is filtered there), so its cached column list holds no
  filter condition and is not keyed by the filter hash.
- There is no query cache, so `testQueryCacheDependsOnFilters` is pending
  and `setQueryCacheLifetime()` in the ChangeFiltersTest is left out; the
  query parses its DQL each time, so it sees the current filters.
- SQLFilterTest's mocked entity managers and connections are a real
  entity manager on SQLite; the expected `setFiltersStateDirty()` calls
  are checked through `isClean()`. The assertion messages with the last
  logged query (`generateMessage()`) are plain: there is no SQL logger.

Filters and DQL pending: the SQLFilterTest company tests (16: joined and
single table inheritance persisters are not ported);
`SwitchContextTest::testSwitchContext` (single table
inheritance); QueryDqlFunctionTest (29: the `CompanyManager` model needs
the joined inheritance persister and schema).

Persistence, not reproduced and so not claimed:

- The DBAL is async, so the entity manager, unit of work, persisters,
  repositories and the collection methods that load (`count()`, `get()`,
  `contains()`, ...) are `async`; PHP's on-the-spot lazy loading is an
  explicit `initialize_object()`. A lazy proxy is an `EntityObject` ghost
  with its identifier set; `EntityManager::find()` initializes a proxy it
  finds in the identity map (PHP does on the first property access).
- The entity manager takes the `Configuration` by value: changes after
  `create()` are not seen, except the filters (`addFilter()` takes
  `&self`; the clones share them, as PHP shares the object). The platform is `SQLitePlatform`. A pessimistic
  lock runs no `SELECT 1 ... FOR UPDATE` query (SQLite has no lock
  clause), and `find()` with a pessimistic lock loads as `NONE` does.
- The persister does not join inverse one-to-one or EAGER to-one
  associations into its SELECT; `createEntity()` loads them with a second
  query (ExtraLazyCollectionTest's `testSliceInverseCollection` and
  `testGetIndexByManyToManyInverseSide` count these queries, and are
  pending). The second level cache, the metadata cache and the entity
  listener resolver are not ported; partial objects are not lazy
  ghosts of their selected fields.
- Entity methods (lifecycle callbacks, entity listeners) are bodies
  registered by class and method name (`object::register_method()`,
  `register_listener_method()`), what a compiler would emit. A custom
  repository class is a name: the repository is an `EntityRepository`
  that reports it. `__call()` magic finders are `EntityRepository::call()`.
- `wrapInTransaction()` takes an async closure; `getReference()` is
  async: for a class with subclasses it calls `find()`, as PHP does.
- The inheritance persisters are one `BasicEntityPersister` whose
  `PersisterKind` (`kind()`) picks the `SingleTablePersister` or
  `JoinedSubclassPersister` overrides; `getOwningTable()` takes the
  entity manager.

Queries and hydration, not reproduced and so not claimed:

- The query keeps no parser result: it parses its DQL each time it needs
  the SQL or executes (the parser result is not `Send`; PHP keeps it and
  the query cache, which is not ported). There is no result cache,
  hydration cache or second level cache, so their setters and the
  cache tests are not ported.
- A result is read whole (the DBAL port fetches all rows): `toIterable()`
  returns the hydrated values in a `Vec` and the `onClear` listener of
  an iteration is not registered. `toIterable()` with the generator keys
  (the index-by keys, else 0, 1, ...) is `to_iterable_with_keys()`;
  `Query::to_iterable_array()` stands for
  `iterator_to_array($query->toIterable(), true)`.
- Enum fields keep their backing values in the entity data of the
  object hydrators; the simple object hydrator checks them and the enum
  property accessors build the cases. The array hydrator, scalar
  results and `NEW` arguments build the cases with `buildEnum()`, as
  PHP's results show them. A backed enum is a `php_value::EnumCase`
  with a value (or an object reporting `instance_of("BackedEnum")`).
- EnumTest reads property attributes through the recorded
  `property_attributes` (PHP's `ReflectionProperty::getAttributes()`).
- TypeTest's `Europe/Berlin` dates compare at their fixed offset
  (+02:00); `testArray` and `testObject` are `n/a` (skipped upstream on
  DBAL 4). A `NEW` object's class registers its constructor with
  `register_new_object_class()` (positional values) or
  `register_new_object_class_with_arguments()` (the keyed arguments: a
  string key is a named argument) (Rust cannot instantiate a class by
  name). An entity as a `NEW` argument is hydrated before the object is
  built (the gathering is split around the async `getEntity()`).
  NewOperatorTest's DTOs are generic objects whose registered
  constructors bind positional, named and variadic arguments and coerce
  scalars as PHP does for a caller without `strict_types`. Custom hydration
  modes are not ported (`invalid_hydration_mode`), nor are `ClassMetadata`
  objects as parameter values.
- The query setters take `&self` and return `&Self` (PHP's fluent
  `static`); results are `Value`s.
- Reading a property of a lazy object does not load it: the tests call
  `initialize_object()` where PHP's property read would.
- Criteria: `EntityRepository::matching()` clones the `Criteria` (PHP
  keeps the object). `PersistentCollection::matching()` and
  `EntityRepository::matching()` give a `Value`: an
  `ArrayCollectionObject` or a `LazyCriteriaCollection`;
  `collection_object::matching()`/`slice()` dispatch on the value. The
  collections that load (`LazyCriteriaCollection`, `PersistentCollection`)
  are `async`; their `php_count()` and `traverse()` (PHP's `count()` and
  `foreach`) see only an initialized collection or a cached count, and a
  synchronous `add()` on a `LazyCriteriaCollection` gives a
  `LogicException`. The persisters are reached through the
  `persisters::EntityPersister` trait (`countCriteria()`, `loadCriteria()`,
  `exists()`, `getSelectConditionStatementSQL()`), which the unit tests
  mock. `getSelectSQL()`/`getCountSQL()` take a `PersisterCriteria` (an
  array or a `Criteria`), and `getManyToManyCollection()`/
  `getOneToManyCollection()` give an array keyed by the `indexBy` field.
- `ResultSetMappingBuilder` dereferences to its `ResultSetMapping`;
  `into_result_set_mapping()` hands it to a native query. A
  `NativeQuery`'s `NEW` objects register their constructor with
  `register_new_object_class()`, as the DQL ones do.

Persistence and query pending: `EntityRepositoryTest::testDefaultRepositoryClassName`
(the DDC753 models and changing the configuration after `create()`);
ExtraLazyCollectionTest's `testSliceInverseCollection` and
`testGetIndexByManyToManyInverseSide` (the inverse one-to-one join, see
above); `NativeQueryTest::testFluentInterface` (the result cache);
Functional `PersistentCollectionTest` (its models extend persistence's
`PersistentObject`, which is not recorded); the unit
`PersistentCollectionTest` (13: mocked persisters and iterator
protocol, not part of the criteria work); the criteria and extra-lazy
tests of the Ticket, filter, enum, inheritance and second level cache
test files (GH7767, GH7836, DDC3719, DDC1885, GH7864, SQLFilterTest,
the `Cache\Persister` tests); the query and result cache
tests of Query QueryTest and `testProcessParameterValueClassMetadata`;
`Functional\QueryTest::testJoinPartialObjectHydration` (partial objects)
and `testQueryBuilderWithStringWhereClauseContainingOrAndConditionalPrimary`
(the query builder); `AbstractHydratorTest`'s `onClear` listener tests;
`CustomHydratorTest` (custom hydration modes);
`BasicFunctionalTest::testManyToOneFetchModeQuery` (the inverse
one-to-one join, see above); the remaining DQL-based LifecycleCallbackTest, ManyToManyEventTest and
`OneToManyUnidirectionalAssociationTest::testPersistOwningInverseCascade`
tests (not yet translated); the UnitOfWorkTest tests that read
`EntityPersisterMock`'s recorded inserts and its mocked id generator
type, and `UnitOfWorkTest::testItPreservesTheOriginalExceptionOnRollbackFailure`
(`Thrown` has no previous exception; a failing rollback's error replaces
the commit's).
