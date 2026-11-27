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
| `Mapping\ClassMetadata`: the mapping API (`mapField()`, the association `map*()` methods, overrides, inheritance, discriminator, lifecycle callbacks (`invokeLifecycleCallbacks()`), entity listeners, id generators, sequences, `__sleep()`/`unserialize()`, `newInstance()`), with the `MappingException` texts; the metadata is shared as `ClassMetadata` (an `RwLock` over `ClassMetadataInfo`) | `Mapping/ClassMetadata.php`, `MappingException.php`, `Exception/*.php` | ClassMetadataTest (69) |
| The mapping value objects: `FieldMapping`, `AssociationMapping` and its subclasses, `JoinColumnMapping`, `JoinTableMapping`, `EmbeddedClassMapping`, `DiscriminatorColumnMapping` (`fromMappingArray()`, `toArray()`, `ArrayAccess`, `__sleep()`) | `Mapping/*Mapping.php` | the `*MappingTest` files and TableMappingTest (23) |
| The mapping attributes (`Entity`, `Table`, `Column`, `Id`, `GeneratedValue`, the association and join attributes, `OrderBy`, overrides, lifecycle callbacks, ...) with PHP's constructor defaults; `AttributeReader` and `AttributeDriver` over a registry that stands in for PHP attributes (`ClassAttributes`, `register_attributes()`: what a derive macro would emit) | `Mapping/*.php` (attributes), `Driver/AttributeReader.php`, `Driver/AttributeDriver.php` | AttributeReaderTest (5), AttributeDriverTest with MappingDriverTestCase (62); test models recorded from PHP 8.4.27 reflection (`tests/data/models.json`, oracle/recorders/orm_models.php) |
| `ClassMetadataFactory` on persistence's `AbstractClassMetadataFactory`: inheritance, embeddables, id generator completion (identity preferences, DBAL 4 PostgreSQL identity, Oracle sequences), default discriminator map, runtime validation | `Mapping/ClassMetadataFactory.php` | ClassMetadataFactoryTest (12), BasicInheritanceMappingTest (14), ClassMetadataLoadEventTest (1) |
| The builders (`ClassMetadataBuilder`, `FieldBuilder`, `EmbeddedBuilder`, the association builders, `EntityListenerBuilder`) | `Mapping/Builder/*.php` | ClassMetadataBuilderTest (38), FieldBuilderTest (1) |
| Naming strategies (`DefaultNamingStrategy`, `UnderscoreNamingStrategy`), quote strategies (`DefaultQuoteStrategy`, `AnsiQuoteStrategy`), typed field mappers (`DefaultTypedFieldMapper`, `ChainTypedFieldMapper`) | `Mapping/*NamingStrategy.php`, `*QuoteStrategy.php`, `*TypedFieldMapper.php`, `Internal/SQLResultCasing.php` | NamingStrategyTest (6), QuoteStrategyTest (8), AnsiQuoteStrategyTest (8), DefaultQuoteStrategyTest (2), TypedFieldMapperTest (1, 12 rows), TypedEnumFieldMapperTest (1) |
| The property accessors (`RawValue`, `ObjectCast`, `TypedNoDefault`, `Readonly`, `Enum`, `Embeddable`, the factory) and the legacy `ReflectionReadonlyProperty`/`ReflectionEmbeddedProperty` | `Mapping/PropertyAccessors/*.php`, `ReflectionReadonlyProperty.php`, `ReflectionEmbeddedProperty.php` | the PropertyAccessors tests (10), ReflectionReadonlyPropertyTest (3), ReflectionEmbeddedPropertyTest (2, 6 rows each) |
| `Query\Parser`: the DQL grammar (`QueryLanguage()` down to the terminals), the deferred validations (identification and result variables, partial objects, path expressions, `NEW` objects), the built-in and custom string, numeric and datetime functions, the `QueryException` syntax and semantical texts; `AbstractSchemaName()`, `match()` | `Query/Parser.php`, `Query/QueryException.php`, `Query/AST/ASTException.php` | LanguageRecognitionTest (112 of 113), ParserTest (6) |
| `Query\AST`: the nodes (as `ast::Node`, double-dispatched to the walker) and the functions (`ABS` ... `UPPER`, `IDENTITY`, `SIZE`, `DATE_ADD`/`DATE_SUB`, the bit functions); `ExpressionWithReturnType` and the deprecated `TypedExpression` (deprecation `https://github.com/doctrine/orm/pull/12543`) | `Query/AST/*.php`, `Query/AST/Functions/*.php` | the SqlGeneration tests; TypedExpressionDeprecationTest (2); Functional/QueryDqlFunctionTest (28 of 29, on SQLite) |
| `Query\SqlWalker` and `SqlOutputWalker`: the SQL of SELECT, UPDATE and DELETE statements on the platform, table aliases, class table and single table inheritance, ordered collections, filters, lock hints, `HINT_FORCE_PARTIAL_LOAD`, limit and offset, custom DBAL types' `convertToPHPValueSQL()`, the result set mapping the walker fills; `OutputWalker::getFinalizer()` | `Query/SqlWalker.php`, `Query/SqlOutputWalker.php`, `Query/OutputWalker.php` | SelectSqlGenerationTest (172 of 185), UpdateSqlGenerationTest (18), DeleteSqlGenerationTest (28), SqlWalkerTest (2) |
| `TreeWalker`, `TreeWalkerAdapter`, `TreeWalkerChain`: walkers that change the AST (`&mut`) before the output walker; `setQueryComponent()`'s check | `Query/TreeWalker.php`, `TreeWalkerAdapter.php`, `TreeWalkerChain.php` | CustomTreeWalkersTest (5), CustomTreeWalkersJoinTest (2) |
| `ParserResult`, the executors and finalizers (`SqlExecutor` with its kind and SQL statements, `execute()` on the connection with bound parameters, the multi-table executors' temporary tables, `SingleSelectSqlFinalizer`, `PreparedExecutorFinalizer`), the deprecation `https://github.com/doctrine/orm/pull/11188` | `Query/ParserResult.php`, `Query/Exec/*.php` | ParserResultTest (5), Functional QueryTest |
| `Query` and `AbstractQuery`: `EntityManager::createQuery()`, DQL and state, parameters (`setParameter()` with name normalization, `setParameters()` from parameters or an array, `processParameterValue()` for entities, collections, arrays and backed enums), the parameter count checks, `setFirstResult()`/`setMaxResults()`, hints and default hints, lock modes (`TransactionRequiredException`), `setFetchMode()`, `getSQL()`, `execute()`, `getResult()`, `getArrayResult()`, `getScalarResult()`, `getSingleColumnResult()`, `getSingleResult()`, `getOneOrNullResult()`, `getSingleScalarResult()` (`NoResultException`, `NonUniqueResultException`), `toIterable()`, `free()`, `clone` | `Query.php`, `AbstractQuery.php`, `NoResultException.php`, `NonUniqueResultException.php`, `UnexpectedResultException.php` | Query QueryTest (32 of 39), Functional QueryTest (39 of 41), QueryIterableTest (4), QueryParameterTest (6), IndexByAssociationTest (5), TypeTest (9 of 11), TypeValueSqlTest (4), EnumTest (24), on SQLite |
| The hydrators: `AbstractHydrator` (column information, `gatherRowData()`, `gatherScalarRowData()`, `buildEnum()`, `registerManaged()`), `ObjectHydrator` (fetch joins, collections, index by, `NEW` objects with named arguments and entities as arguments, mixed results, discriminators, proxies, `HydrationException`s), `ArrayHydrator`, `ScalarHydrator`, `SingleScalarHydrator`, `ScalarColumnHydrator`, `SimpleObjectHydrator` (discriminators, joined inheritance fields, enum checks with `MappingException::invalidEnumValue()`) | `Internal/Hydration/*.php`, `Exception/MultipleSelectorsFoundException.php` | ObjectHydratorTest (31), ArrayHydratorTest (19), ScalarHydratorTest (3), SingleScalarHydratorTest (4), ScalarColumnHydratorTest (4), SimpleObjectHydratorTest (6), AbstractHydratorTest (2 of 6), ResultSetMappingTest (3), Functional/NewOperatorTest (38, on SQLite) |
| `Query\Filter`: `SQLFilter` (`setParameter()` with the inferred or given type, `setParameterList()`, `getParameter()` quoted by the connection, `getParameterList()` quoted and comma-joined, `hasParameter()`, `__toString()` as PHP's `serialize()` of the ksorted parameters, `getConnection()`, the `FilterException` texts), `FilterCollection` (`enable()`, `disable()`, `suspend()`, `restore()`, `getFilter()`, `has()`, `isEnabled()`, `isSuspended()`, `isClean()`, `getHash()`, `setFiltersStateDirty()`, the InvalidArgumentException texts); `Configuration::addFilter()`/`getFilterClassName()`; `EntityManager::getFilters()`, `isFiltersStateClean()`, `hasFilters()`; the filter conditions in the SQL walker and in the persister's `getSelectSQL()`, `getCountSQL()` and `exists()`, on the root entity for the inheritance persisters (`SingleTablePersister::generateFilterConditionSQL()`, the `JoinedSubclassPersister` select and count SQL); the many-to-many persister's `getFilterSql()` (the join of the target table `te` for the extra-lazy `count()`, `contains()`, `containsKey()` and `get()`) | `Query/Filter/SQLFilter.php`, `Query/Filter/Parameter.php`, `Query/Filter/FilterException.php`, `Query/FilterCollection.php`, `Configuration.php`, `EntityManager.php`, `Persisters/Entity/BasicEntityPersister.php`, `Persisters/Entity/SingleTablePersister.php`, `Persisters/Entity/JoinedSubclassPersister.php`, `Persisters/Collection/ManyToManyPersister.php` | Functional/SQLFilterTest (48 of 49), FilterCollectionTest (9), ConfigurationTest::testAddGetFilters, the SwitchContextWithFilter ChangeFiltersTest (3) and the SwitchContextWithFilterAndIndexedRelation ChangeFiltersTest (1), on SQLite |
| `NativeQuery` and `EntityManager::createNativeQuery()` | `NativeQuery.php` | NativeQueryTest (20), Query NativeQueryTest (1) |
| The query cache: `Query` keeps the parser result of a clean query and caches it in a PSR-6 pool (`setQueryCache()`, `useQueryCache()`, `setQueryCacheLifetime()`, `expireQueryCache()`, the cache id of the DQL, hints, platform, filters, hydration mode and parameter types); `Configuration::setQueryCache()`/`getQueryCache()`, and the result, hydration and metadata cache pools' setters and getters | `Query.php`, `Configuration.php` | QueryCacheTest (6), ConfigurationTest (`testSetGetQueryCache`, `testSetGetHydrationCache`, `testSetGetMetadataCache`), SQLFilterTest::testQueryCacheDependsOnFilters, PaginationTest::testDifferentResultLengthsDoNotRequireExtraQueryCacheEntries, LimitSubqueryOutputWalkerTest::testParsingQueryWithDifferentLimitOffsetValuesTakesOnlyOneCacheEntry |
| `TopologicalSort` and `StronglyConnectedComponents` (commit order, cycle breaking on optional edges, `CycleDetectedException`) | `Internal/TopologicalSort.php`, `Internal/StronglyConnectedComponents.php` | TopologicalSortTest, StronglyConnectedComponentsTest |
| `EntityManager` (async where it reaches the database): `create()` (`MissingMappingDriverImplementation`), `find()`/`find_with_lock()` (identifier normalization, `invalidCompositeIdentifier`, `MissingIdentifierField`, `UnrecognizedIdentifierFields`, lock requirements: `OptimisticLockException::notVersioned`, `TransactionRequiredException`), `getReference()`, `persist()`, `remove()`, `refresh()`/`refresh_with_lock()` (`TransactionRequiredException` for a pessimistic lock), the deprecation `https://github.com/doctrine/orm/pull/12548` for a null lock mode of `find()` and `refresh()`, `detach()`, `contains()`, `clear()`, `close()` (`EntityManagerClosed`), `flush()`, `wrapInTransaction()`, `beginTransaction()`/`commit()`/`rollback()`, `getRepository()`, `lock()` (optimistic with an integer or datetime version); `Configuration` (`setDefaultRepositoryClassName()`'s `InvalidEntityRepository`, the proxy settings with the deprecation `https://github.com/doctrine/orm/pull/12005`, the custom functions and hydration modes, the class metadata factory name, the naming and quote strategies, the typed field mapper) | `EntityManager.php`, `Configuration.php`, `Proxy/ProxyFactory.php` (native lazy ghosts) | EntityManagerTest (15 of 18), ProxyFactoryTest (7 of 11, 2 skipped upstream), ConfigurationTest (20), BasicFunctionalTest (33 of 34), EntityRepositoryTest (35), IdentityMapTest (5), CompositePrimaryKeyTest (8 of 9, 1 skipped upstream), OneToOneEagerLoadingTest (9), OneToOneBidirectionalAssociationTest (7), ReadOnlyTest (7), Locking/OptimisticTest (11), Locking/LockTest (13 of 16), AdvancedDqlQueryTest (12), GH8011Test (12), Ticket/DDC117Test (20, composite keys of associations), ReferenceProxyTest (12 of 13), ValueObjectsTest (11 of 12, embeddables), the Ticket tests on the cms, ecommerce, generic, quote and legacy models and on their own models (255: DDC1040, DDC1043, DDC1113, DDC1129, DDC1181, DDC1193, DDC1225, DDC1228, DDC1238, DDC1301, DDC1306, DDC142, DDC1452, DDC1545, DDC1643, DDC1666, DDC1778, DDC1843, DDC1885, DDC1918, DDC2074, DDC2519, DDC2655, DDC2790, DDC3160, DDC657 (4 of 5), DDC736, DDC748, DDC767, DDC812, DDC849, DDC949, DDC69, DDC144, DDC192, DDC199, DDC211, DDC237, DDC279, DDC309, DDC345, DDC353, DDC371, DDC422, DDC425, DDC440, DDC444, DDC448, DDC493, DDC512, DDC513, DDC522, DDC531, DDC588, DDC599, DDC618, DDC633, DDC656, DDC698, DDC719, DDC735, DDC809, DDC837, DDC881, DDC960, DDC992, DDC1250, DDC1300, DDC1335, DDC1400, DDC1430, DDC1436, DDC1454, DDC1458, DDC1461, DDC1514, DDC1515, DDC1526, DDC1548, DDC1595, DDC1654, DDC1707, DDC1719, DDC1757, DDC1787, DDC1925, DDC2084, DDC2106, DDC2175, DDC2214, DDC2660, DDC2692, DDC2759, DDC2780, DDC2895, DDC2931, DDC2996, DDC3042, DDC3123, DDC3170, DDC3223, DDC3330, DDC3644, DDC4024, DDC6558, DDC6573, DDC1080, DDC1163, DDC2138, DDC2252, DDC2306, DDC2346, DDC2350, DDC2575, DDC258, DDC2775, DDC3033, DDC3597, GH10132, GH10334, GH10336, GH10348, GH10387, GH10450, GH10454, GH10531, GH10532, GH10566, GH10625, GH10752, GH10808, GH10880, GH10889, GH10912, GH10913, GH11058, GH11135, GH11163, GH11199, GH11341, GH11487, GH11524, GH11982 (1 of 2), GH12172, GH12183, GH12254, GH12438, GH5742, GH5762, GH5998, GH6029, GH6123, GH6362, GH6394, GH6402, GH6464, GH6499, GH6499OneToManyRelationship, GH6499OneToOneRelationship, GH6531, GH6682, GH6699, GH6740, GH6937, GH7006, GH7012, GH7068) |
| `UnitOfWork` (UnitOfWorkTest runs on a stubbed driver connection, as upstream's mocks): change sets (scalars, to-one, collections, `recomputeSingleEntityChangeSet()`), commit order by `TopologicalSort`, identity map (`EntityIdentityCollisionException`), cascades (persist, remove, refresh, detach), orphan removal, collection deletions and updates, entity states, `createEntity()` (lazy to-one proxies, deferred eager loads), the ORMInvalidArgumentException texts (`newEntityFoundThroughRelationship`, `invalidAssociation`, ...) | `UnitOfWork.php`, `ORMInvalidArgumentException.php`, `Exception/*.php`, `EntityNotFoundException.php`, `OptimisticLockException.php` | BasicFunctionalTest, UnitOfWorkTest (121; `EntityPersisterMock` is a `CommitPersister` set with `UnitOfWork::set_entity_persister()`), the OneToMany*/ManyToMany* association tests (37), OneToManyOrphanRemovalTest (5), LifecycleCallbackTest (12 of 13, the thirteenth row is a model callback named `testCallback`) |
| `PersistentCollection` over collections' `ArrayCollection` (lazy loading, snapshot, insert/delete diffs, dirty flag, orphan scheduling), and an `ArrayCollection` property object; `EXTRA_LAZY` collections (`count()`, `contains()`, `containsKey()` and `get()` of an `indexBy` collection, `slice()`, `first()` without loading the collection); `matching()` (in memory when initialized or dirty, else the one-to-many criteria on the back reference with the association's ordering, a `LazyCriteriaCollection` when extra lazy, or the many-to-many persister's criteria query) | `PersistentCollection.php` | the association tests, ManyToManyBasicAssociationTest (22), OneToManyBidirectionalAssociationTest (12), ExtraLazyCollectionTest (37), PersistentCollectionCriteriaTest (2), the ValueConversionType extra-lazy and criteria matching tests (14), the ValueConversionType many-to-many, one-to-many and one-to-one tests (51, the composite one-to-ones through the inverse one-to-one join), PersistentCollectionTest (13) |
| `BasicEntityPersister` (inserts with identity ids, updates with versions, deletes with join table rows, `load()`, `loadAll()`, `loadById()`, `count()`, `exists()` (with a `Criteria`), `refresh()`, the SELECT/INSERT/COUNT SQL, the joins of the inverse one-to-one and EAGER to-one associations into the SELECT (hydrated by the object hydrator), criteria with `IN` and `NULL`, `loadCriteria()`, `countCriteria()`, `expandCriteriaParameters()`, the comparison operators of a `Criteria` (`LIKE` for `CONTAINS`/`STARTS_WITH`/`ENDS_WITH`, `IS [NOT] NULL`, `IN`/`NOT IN` and `1=0` for an empty list), `InvalidOrientation`, `UnrecognizedField`, `InvalidFindByCall`, `MatchingAssociationFieldRequiresObject`), `SqlExpressionVisitor`, `SqlValueVisitor`, `OneToManyPersister` and `ManyToManyPersister` (`delete()`, `update()`, the extra-lazy `count()`, `slice()`, `containsKey()`, `contains()`, `get()`, and `loadCriteria()` of a many-to-many collection), `PersisterHelper`, `IdentifierFlattener`, the simple object hydrator; the inheritance persisters as kinds of `BasicEntityPersister` (`PersisterKind`): `AbstractEntityInheritancePersister` (the discriminator value in the insert data, the declaring class's column aliases), `SingleTablePersister` (the root table alias, the discriminator column and the subclasses' columns, the discriminator condition), `JoinedSubclassPersister` (inserts into the root table then the sub tables, updates and versions per owning table, deletes from the root table, the parent and subclass joins, the owning table of a field), `OneToManyPersister::deleteJoinedEntityCollection()` (the temporary id table) | `Persisters/Entity/BasicEntityPersister.php`, `Persisters/Entity/AbstractEntityInheritancePersister.php`, `Persisters/Entity/SingleTablePersister.php`, `Persisters/Entity/JoinedSubclassPersister.php`, `Persisters/SqlExpressionVisitor.php`, `Persisters/SqlValueVisitor.php`, `Persisters/MatchingAssociationFieldRequiresObject.php`, `Persisters/Collection/*Persister.php`, `Utility/*.php`, `Internal/Hydration/SimpleObjectHydrator.php` | EntityRepositoryTest, EntityRepositoryCriteriaTest (6), BasicFunctionalTest, ManyToManyBasicAssociationTest (query counts), ExtraLazyCollectionTest, SqlExpressionVisitorTest (1), SingleTableInheritanceTest (22), ClassTableInheritanceTest (16), ClassTableInheritanceSecondTest (2), MappedSuperclassTest (1), OneToOneSingleTableInheritanceTest (1), OrderedJoinedTableInheritanceCollectionTest (1), JoinedTableCompositeKeyTest (2), SingleTableCompositeKeyTest (2), the Ticket tests DDC168, DDC1655 (2), DDC6303 (2), DDC832 (6), GH11500, GH11501, DDC331, DDC1041, DDC163, DDC3719, Persisters/BasicEntityPersisterTypeValueSqlTest (11) |
| `EntityRepository` (`find()`, `findAll()`, `findBy()`, `findOneBy()`, `count()`, `__call()` magic finders with `InvalidMagicMethodCall`, `matching()`, `createResultSetMappingBuilder()`), `DefaultRepositoryFactory` | `EntityRepository.php`, `Repository/*.php` | EntityRepositoryTest (56) |
| `LazyCriteriaCollection`: the cached count, `isEmpty()` and `contains()` without loading, `matching()` in memory after one load | `LazyCriteriaCollection.php` | LazyCriteriaCollectionTest (7), EntityRepositoryCriteriaTest |
| Lifecycle events: `ListenersInvoker` (callbacks, entity listeners, event manager), the event args classes, `PreUpdateEventArgs`; `DefaultEntityListenerResolver` (`Configuration::setEntityListenerResolver()`, the instances outlive `UnitOfWork::clear()`) | `Event/*.php`, `Mapping/EntityListenerResolver.php`, `Mapping/DefaultEntityListenerResolver.php` | LifecycleCallbackTest, EntityListenersTest (10), PostLoadEventTest (9 of 10), EntityListenerResolverTest (4) |
| `SchemaTool::createSchema()`/`dropSchema()`/`getSchemaFromMetadata()`/`updateSchema()`/`getUpdateSchemaSql()` for no, single table and joined (class table) inheritance (columns, join columns and tables, indexes, unique constraints replacing the indexes they fulfill but fulltext and spatial ones, foreign keys, the child tables' primary keys and `ON DELETE CASCADE` foreign keys to the root table); the `ToolEvents` (`postGenerateSchemaTable`, `postGenerateSchema`) and their event args (`setSchema()`, `setClassTable()`) | `Tools/SchemaTool.php`, `Tools/ToolEvents.php`, `Tools/Event/*.php` | the functional tests' schemas, CompanySchemaTest (2), Tools/SchemaToolTest (33), SchemaTool/DBAL483Test (1) |
| `SchemaValidator::validateMapping()`/`validateClass()`: the association checks (unknown targets, mapped superclasses, mappedBy/inversedBy consistency, join columns against the identifiers, ordering fields, discriminator maps) and the property type checks against the built-in DBAL types and enums | `Tools/SchemaValidator.php` | Tools/SchemaValidatorTest (17) |
| `ResultSetMapping` | `Query/ResultSetMapping.php` | ResultSetMappingTest (3); the persister's mappings; the SQL walker's result mappings |
| `ResultSetMappingBuilder`: root and joined entities from class metadata (the column renaming modes, single table inheritance checks, column conflicts), `generateSelectClause()` with table aliases and custom types' `convertToPHPValueSQL()`, `__toString()` | `Query/ResultSetMappingBuilder.php` | NativeQueryTest, EntityRepositoryTest |
| `QueryBuilder`: the DQL parts (`add()` with its `$append` check and the deprecation `https://github.com/doctrine/orm/pull/12051` for numeric join keys, `getDQLPart(s)()`, `resetDQLPart(s)()`), `select()`/`addSelect()`/`distinct()`, `delete()`/`update()` (the alias check), `from()`/`indexBy()`, the joins (root aliases), `set()`, `where()`/`andWhere()`/`orWhere()`, `groupBy()`, `having()` and its variants, `orderBy()`/`addOrderBy()` (the deprecation `https://github.com/doctrine/orm/issues/11313` for an order that is not a `SortDirection`, `Invalid sort direction`), `addCriteria()`, the root aliases and entities, parameters (`setParameter()` updates a parameter of the same normalized name, `createNamedParameter()`), first and max results (no limit for DELETE and UPDATE), hints, the second-level cache options, `getDQL()` (cached until a part changes), `getQuery()`, `__clone()`; `QueryType`; `EntityManager::createQueryBuilder()`, `getExpressionBuilder()`, `EntityRepository::createQueryBuilder()` | `QueryBuilder.php`, `QueryType.php`, `EntityManager.php`, `EntityRepository.php` | QueryBuilderTest (108 of 109), ExprTest's subquery tests (5), Functional/QueryBuilderParenthesisTest (2, on SQLite) |
| `QueryExpressionVisitor`: a `Criteria`'s comparisons and composites as `Expr` objects with their parameters (the alias prefix, `_`-joined parameter names made unique by the parameter count, `CONTAINS`/`STARTS_WITH`/`ENDS_WITH` as `LIKE`, `MEMBER OF`, `IS NULL` for a null value), the `RuntimeException` texts | `Query/QueryExpressionVisitor.php` | QueryExpressionVisitorTest (6) |
| `Tools\Pagination`: `CountWalker` (`HINT_DISTINCT`, the HAVING and two-FROM texts), `CountOutputWalker`, `LimitSubqueryWalker` (the identifier type hint, `HINT_ENABLE_DISTINCT`), `LimitSubqueryOutputWalker` (`SELECT DISTINCT <ids> FROM (...) dctrn_result`, the ORDER BY columns added as hidden selects, the inner ordered query; the `ROW_NUMBER()` SQL for PostgreSQL, SQL Server, Oracle and DB2), `WhereInWalker`, `RootTypeWalker`, `RowNumberOverFunction` (`RowNumberOverFunctionNotEnabled`), the RuntimeException texts; `PaginatorQuery` (`getCountQuery()`, `cloneQuery()`, `appendTreeWalker()`, `unbindUnusedQueryParams()`, `useOutputWalker()`, `getResultForOffset()` with the fetch-join ID subquery + WHERE IN path and the output-walker path, `convertWhereInIdentifiersToDatabaseValues()`); `Paginator` (`count()`, `getIterator()`); `OffsetPaginator` and `PaginatorInterface` (`paginate()`, its InvalidArgumentException text); `Window`, `Page`, `WindowPage` (the LogicException texts); cursor pagination: `Cursor` (`encodeToString()` as URL-safe Base64 JSON, `fromEncodedString()` with strict Base64, JSON depth 2 and the scalar checks, `toArray()`), `CursorItem`, `InvalidCursor` (with its `JsonException` as `getPrevious()`), `CursorWalker` (the ORDER BY directions, reversed for a previous page; the `dctrn_cursor_<n>` condition `(a > ?) OR (a = ? AND (...))` added to the WHERE clause; the ORDER BY items hint; the LogicException text), `OrderDirection`, `CursorOrderByItem`, `CursorPaginator` (the `$limit + 1` rows, the ID subquery + WHERE IN path for a query that produces duplicates, reversed items for a previous cursor, the cursor values from the entity's fields, an association's identifier, converted by the field's type), `CursorPage` (the memoized COUNT, the page flags, the next and previous cursors and their LogicException texts) | `Tools/Pagination/{CountWalker,CountOutputWalker,LimitSubqueryWalker,LimitSubqueryOutputWalker,WhereInWalker,RootTypeWalker,RowNumberOverFunction,Paginator,PaginatorQuery,PaginatorInterface,OffsetPaginator,Page,Window,WindowPage}.php`, `Tools/Pagination/Exception/RowNumberOverFunctionNotEnabled.php`, `Tools/Pagination/{Cursor,CursorItem,CursorOrderByItem,CursorPage,CursorPaginator,CursorWalker,OrderDirection}.php`, `Tools/Pagination/Exception/InvalidCursor.php` | CountWalkerTest (8), WhereInWalkerTest (1), LimitSubqueryWalkerTest (10), RootTypeWalkerTest (1), CountOutputWalkerTest (4 of 5), LimitSubqueryOutputWalkerTest (13 of 27), PaginatorTest (3), OffsetPaginatorTest (3), WindowTest (10 of 11), WindowPageTest (16), Functional/PaginationTest (56 of 57, on SQLite), CursorTest (12 of 13), CursorWalkerTest (14), CursorPaginatorTest (25) |
| Second-level cache: the cache keys and entries, `DefaultRegion` and `FileLockRegion` over an in-memory PSR-6 pool (`cache::pool::ArrayAdapter`, for symfony/cache's `ArrayAdapter`), `UpdateTimestampCache`, `CacheLogger`, `CacheLoggerChain`, `StatisticsCacheLogger`, `CacheConfiguration`, `RegionsConfiguration`, `TimestampQueryCacheValidator`, `DefaultEntityHydrator`, `DefaultCollectionHydrator`, the cached entity persisters (`ReadOnly`, `NonStrictReadWrite`, `ReadWrite` with region locks) over `EntityPersister` (`persisters::EntityPersisterInterface`), the cached collection persisters over `CollectionPersister` (`persisters::collection::CollectionPersister`, with `UnitOfWork::getCollectionPersister()`), `DefaultCacheFactory`, `DefaultCache`, `DefaultQueryCache`, `EntityManager::getCache()`, a cacheable query's use of the query cache (`AbstractQuery::executeUsingQueryCache()`, `getHash()`, `getTimestampKey()`, `Query::HINT_CACHE_EVICT`), the cache exception classes; `Configuration::setSecondLevelCacheEnabled()`/`setSecondLevelCacheConfiguration()`/`getSecondLevelCacheConfiguration()` | `Cache.php`, `Cache/*.php`, `Cache/Region/*.php`, `Cache/Logging/*.php`, `Cache/Persister/Entity/*.php`, `Cache/Persister/Collection/*.php`, `Cache/Exception/*.php`, `Persisters/Entity/EntityPersister.php`, `Persisters/Collection/CollectionPersister.php`, `AbstractQuery.php`, `Query.php` | CacheKeyTest (8), CacheConfigTest (4), CacheLoggerChainTest (4), StatisticsCacheLoggerTest (4), DefaultRegionTest (8), FileLockRegionTest (10), DefaultEntityHydratorTest (7), DefaultCollectionHydratorTest (2), ReadOnlyCachedEntityPersisterTest (24), NonStrictReadWriteCachedEntityPersisterTest (28), ReadWriteCachedEntityPersisterTest (32), ReadOnlyCachedCollectionPersisterTest (8), NonStrictReadWriteCachedCollectionPersisterTest (8), ReadWriteCachedCollectionPersisterTest (18), DefaultCacheTest (13), DefaultCacheFactoryTest (14), DefaultQueryCacheTest (24), ConfigurationTest::testSetGetSecondLevelCacheConfig, Functional/SecondLevelCacheTest (8), SecondLevelCacheQueryCacheTest (26), SecondLevelCacheManyToOneTest (6), SecondLevelCacheOneToManyTest (9), SecondLevelCacheManyToManyTest (5), SecondLevelCacheOneToOneTest (6), SecondLevelCacheRepositoryTest (6), SecondLevelCacheCountQueriesTest (4), SecondLevelCacheCriteriaTest (3), SecondLevelCacheCompositePrimaryKeyTest (3), SecondLevelCacheExtraLazyCollectionTest (1), SecondLevelCacheCompositePrimaryKeyWithAssociationsTest (1), SecondLevelCacheConcurrentTest (2), SecondLevelCacheSingleTableInheritanceTest (8), SecondLevelCacheJoinTableInheritanceTest (7), Performance/SecondLevelCacheTest (8), on SQLite |

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
- Cursor pagination: `CursorPage` does not implement the synchronous
  `Page` trait: `getTotalCount()` (the COUNT query), the cursor getters
  and `getItemsWithCursors()` need the database and are `async`; its
  closures hold a clone of the paginated query. `fromEncodedString()`
  takes the `mixed` value as a `&Value` and returns `InvalidCursor`,
  which keeps the `JsonException` it chains (a `Thrown` has no previous
  exception, so converting it to one drops it). The ORDER BY items reach
  the paginator as `CursorOrderByItem` objects that keep a path
  expression's alias and field (the AST node is not `Send`). The walker
  binds the cursor parameters through the walked `Query` (another
  `DqlQuery` gets none).
  CursorPaginatorTest's stubbed hydrator is the recording connection's
  rows hydrated as `BlogPost` entities (and a `sclr_0` row for the
  COUNT); `testCursorIsFinal` is n/a.
- PaginatorTest and OffsetPaginatorTest replace the mocked connection and
  hydrator with a recording driver connection
  (`tests/common/recording_connection.rs`) that gives queued rows and
  records the bound values; the expected values are those bound after
  the array parameters are expanded (`[2, 1, 10]`).
- Functional/PaginationTest: `testDifferentResultLengthsDoNotRequireExtraQueryCacheEntries`
  sets the query cache `OrmFunctionalTestCase` configures for every
  functional test (the other tests run without one). A data provider's rows run in one
  test. `CustomIdObject`'s `__toString()` is compared through its `id`.
  The models are recorded lines appended to `tests/data/models.json`.
- `LimitSubqueryWalker::IDENTIFIER_TYPE`'s hint holds the DBAL type as
  `TypeObject`, a `php_value` object around the `Arc<dyn Type>`.
- Only `SQLitePlatform` exists: the walker tests on `MySQLPlatform` run on
  it (no `ROW_NUMBER()` and the same `LIMIT ... OFFSET ...`, so the SQL is
  upstream's); the PostgreSQL, Oracle and SQL Server cases are pending.
  `testParsingQueryWithDifferentLimitOffsetValuesTakesOnlyOneCacheEntry`
  runs on a functional `Query`, which has the query cache.

Mapping, not reproduced and so not claimed:

- PHP attributes are a registry (`mapping::attributes`); the test models
  are registered from PHP reflection recorded by
  oracle/recorders/orm_models.php (PHP 8.4.27, the plain
  `php:8.4.27-cli-bookworm` image). The recorder autoloads doctrine/dbal
  at php2rust-doctrine-dbal's pin (`upstream.tsv`) for the custom DBAL
  types and middlewares that test files declare. There are no recorded
  ORM vectors beyond the models.
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
  and `exists()`, and in the joins of the inverse one-to-one and EAGER
  to-one associations; its cached column list is rebuilt when the filter
  hash changes.
- The query cache id includes the filter hash when filters were used,
  and the query parses again when the filter collection is dirty, as
  upstream does (`setQueryCacheLifetime()` in the ChangeFiltersTest is
  still left out).
- SQLFilterTest's mocked entity managers and connections are a real
  entity manager on SQLite; the expected `setFiltersStateDirty()` calls
  are checked through `isClean()`. The assertion messages with the last
  logged query (`generateMessage()`) are plain: there is no SQL logger.

Filters and DQL pending: `SwitchContextTest::testSwitchContext` (single
table inheritance); `QueryDqlFunctionTest::testFunctionSqrt` (the SQLite
library of the DBAL port has no math functions: `no such function:
SQRT`).

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
- The metadata cache is not ported; partial objects are not lazy ghosts
  of their selected fields.
- `PersistentCollection::key()`, `current()` and `next()` are async and
  initialize the collection, as `AbstractLazyCollection`'s do.
  `unserialize(serialize($collection))` is `serialized_copy()`: the
  `__sleep()` properties (`collection`, `initialized`), with no entity
  manager. PersistentCollectionTest runs on a SQLite entity manager: the
  mocked `UnitOfWork::loadCollection()` callbacks are products of a
  persisted cart, and the mocked `scheduleCollectionDeletion()` is
  `isCollectionScheduledForDeletion()`.
- Second-level cache: the pool is in memory only; it keeps a copy of each value, as
  `ArrayAdapter` keeps serialized values. A lock's `uniqid()` takes a
  number from a per-process sequence in place of PHP's combined LCG.
  `FileLockRegion` reads a lock's time from the file's access time. A
  criteria's orderings enter a cached query's hash as their `ASC`/`DESC`
  names. A factory that replaces `getRegion()` builds its persisters
  and query caches with `cache::factory::build_cached_entity_persister()`,
  `build_cached_collection_persister()` and `build_query_cache()`. The
  persister tests' `createMock()` mocks record their calls and are
  checked after the call; `CacheRegionMock`, `TimestampRegionMock` and
  `ConcurrentRegionMock` record the call's key (and entry) or the key's
  hash. A cacheable query's hash serializes its hints as PHP values: a
  lock mode is its integer, the custom walkers are their class names.
  A collection cache entry holds cache keys, so DefaultCacheTest puts
  the keys of the cities `['id' => 11]` and `['id' => 12]` where PHP
  puts the arrays. `DefaultCache::toIdentifierArray()`,
  `DefaultQueryCache::getAssociationValue()` and `Query::getHash()` are
  public (the tests call them by reflection). The functional tests'
  cache pool is per test (PHP shares it between the tests of a run), as
  is the database; a getter that loads a lazy object is
  `initializeObject()`.
  The functional fixture enables the query log after `setUp()`, as
  upstream does. A data provider's rows run in turn in one test, each on
  a new fixture (SecondLevelCacheCountQueriesTest's `READ_WRITE` row puts
  its `FileLockRegion` directory under the system temporary directory,
  named after the test). A new `ResultSetMapping` is a select
  (`$isSelect = true`), so the persisters' mappings can be cached.
  `PersistentCollection::matching()` loads through the cached entity
  persister when the target is cached (`getEntityPersister()`).
  SecondLevelCacheCompositePrimaryKeyWithAssociationsTest leaves the
  GeoNames `City` (not recorded) out of its schema; SecondLevelCacheConcurrentTest's
  factory keeps the `ConcurrentRegionMock`s it returns by region name
  (`assert($region instanceof ConcurrentRegionMock)`). Performance/SecondLevelCacheTest
  builds a `...WithCache` test's entity manager with the cache (PHP
  calls `enableSecondLevelCache()` after `setUp()`, too late for its
  entity manager) and asserts the statement counts: PHP's counts minus
  two for each flush.
- A custom hydration mode is only a name in the `Configuration`.
  `getTypedFieldMapper()` is `None` until one is set (the entity manager
  gives the metadata factory the `DefaultTypedFieldMapper` then, as
  upstream's factory does). `enableNativeLazyObjects(false)` triggers
  its deprecation, then is refused: the port only has native lazy
  objects. EntityManagerTest's pending tests replace the connection's
  `commit()`/`rollBack()` in a subclass (the DBAL connection is a
  struct) or make a lazy ghost of the entity manager.
- `ProxyFactory` (`proxy::ProxyFactory`, `EntityManager::get_proxy_factory()`)
  has only native lazy objects: `getProxy()` is `EntityManager::get_proxy()`
  and `generateProxyClasses()` generates none. PHP's optional constructor
  arguments are `with_proxy_args()`, which triggers the 12005 deprecation
  ("Passing more than just the EntityManager ..."). A proxy that
  `getProxy()` made and the unit of work does not manage loads by the
  identifier set on it, as PHP's lazy ghost initializer captures it.
  ProxyFactoryTest runs on a SQLite entity manager: the mocked persister's
  `loadById()` is the real one, and the failing loads are of an identifier
  the database does not have. Its two tests that disable native lazy
  objects are pending. ConfigurationTest's cache tests
  check the pools' setters and getters.
- The company models' entity listener records each call as the entity
  and the class of the event arguments (EntityListenersTest checks that
  class). PostLoadEventTest's
  `testEventRaisedCorrectTimesWhenOtherEntityLoadedInEventHandler` is
  pending: its listener loads a lazy proxy inside `postLoad()`, and a
  listener's call is synchronous while the load is async.
- `getUpdateSchemaSql()` compares with the introspected schema: the
  connection's schema assets filter is not widened to the tables and
  sequences of the classes, as upstream's `createSchemaForComparison()`
  does for a filter that is set (the DBAL port's configuration has no
  unset filter, and the connection's configuration is not changed through
  the entity manager).
- SchemaToolTest's schemas are read through DBAL's editor API where
  upstream reads it when it exists. A `postGenerateSchemaTable` listener
  reaches the schema and the table through closures (`with_schema()`,
  `with_class_table()`); the foreign keys of a replaced table are not
  added to the schema (upstream adds them to the detached table). A
  configuration change after `create()` is made before it, or on a second
  entity manager. The `StaticPHPDriver` call of a class's static
  `loadMetadata()` is the test's Rust function.
- SchemaValidatorTest's model directories are class locators of the
  recorded classes of their namespace. `schemaInSyncWithMetadata()` and
  `getUpdateSchemaList()` are not ported (no schema updates).
- `clone $entity` has no runtime counterpart: ReferenceProxyTest's
  `clone_entity()` initializes the lazy original (as PHP 8.4 does before
  cloning a lazy object), copies its properties and runs the model's
  `__clone()` body. `testCommonPersistenceProxy` is n/a (skipped upstream
  with native lazy objects). An `indexBy` naming an association's join
  column (DDC117Link's `target_id`) keys the collection by the associated
  entity's referenced identifier, the value of the column
  `addIndexBy()` finds among the meta mappings.
- ValueObjectsTest's `testPartialDqlOnEmbeddedObjectsField` is pending:
  partial objects are not lazy ghosts.
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
- `getSelectColumnsSQL()` and `getSelectConditionSQL()` are public (with
  the entity manager), for BasicEntityPersisterTypeValueSqlTest, which
  calls them by reflection. That test runs on a logged SQLite entity
  manager in place of `EntityManagerMock` and the mocked connections:
  `propertyChanged()` (not ported) is `computeChangeSet()` over the
  registered original data, and the mocked `executeStatement()` and
  `delete()` calls are the logged statements with their bound values
  and types.
- `OptimisticLockException::getEntity()` is not kept: `Thrown` holds a
  class, message and code, so the OptimisticTest failure tests check the
  exception's class. PHPUnit's `#[Depends]` runs the test it depends on
  first, then clears the entity manager.
- PHP's nullable lock mode of `find()` and `refresh()` is
  `impl Into<Option<LockMode>>` on `find_with_lock()` and
  `refresh_with_lock()`: `None` is PHP's deprecated `null`. A datetime
  `#[Version]` field defaults to the DBAL's `CurrentTimestamp` expression
  object.
- The LockTest pessimistic lock tests that upstream skips on SQLite
  (`testLockPessimisticRead`, `testLockPessimisticWrite`,
  `testRefreshWithLockPessimisticWrite`) are `n/a`.
- DDC3123Test's listener reads the private `UnitOfWork::$extraUpdates`
  through `ReflectionProperty`; the port reads it with the additive
  `UnitOfWork::get_scheduled_extra_updates()`.
- The Ticket tests that declare their own models construct them
  without a constructor body: the tests set what the constructor would
  (an `ArrayCollection`, a promoted property), and a model method is
  written out in the test where it does more than set a property.
  `clone` of a proxy (DDC3223) initializes it and copies its properties
  into a new object. A `setUp()` that catches a
  `UniqueConstraintViolationException` or an `Exception`, and a
  `tearDown()` that deletes the rows, have nothing to do: every test has
  a fresh database.
- Upstream's `TestUtil` adds DBAL's `EnableForeignKeys` middleware on
  SQLite; the DBAL port has no middlewares, so the tickets whose
  flush order the foreign keys check (GH10752, GH5742, GH6499 and its
  two relationship tests) run `PRAGMA foreign_keys = ON`
  (`ticket::set_up_with_foreign_keys()`).
- A data provider's rows run in one test (GH10454, GH11199, GH11341;
  GH11341 builds a new database per row). The `assertMatchesRegularExpression()`
  patterns of GH11199, GH11341 and GH6464 are matched by hand (an
  anchored suffix, a substring, or the `\w+` words of GH6464).
- DDC1163Test asserts that `find()` returns the uninitialized proxy in
  the identity map; the port's `find()` initializes it (a property read
  cannot trigger the lazy load), so the test asserts it is initialized.
- `markLazyObjectAsInitialized()` is `EntityObject::set_lazy(false)`.
- The listener-driven GH11524Test's `getTranslation()` on a proxy is
  `initialize_object()` and the model method written out in the test.

Other tickets pending: `DDC657Test::testTicketResult` (it formats a
`DateTime` with `G`, which the runtime's `DateTime::format()` does not
support); DDC1209Test (an identifier that is a `DateTime` subclass with
`__toString()`); DDC2359Test (PHPUnit mocks of the metadata factory);
DDC2387Test (`DatabaseDriver` is not ported); DDC3300Test, GH10473Test
and GH12174Test (`ResolveTargetEntityListener` is not ported);
DDC381Test (`serialize()` of a proxy); DDC2602Test and GH10869Test
(their `postLoad`/`postPersist` listener runs a query or `flush()`: a
listener's call is synchronous, the query is async); GH12403Test
(UnitOfWork's partial-load tracking of native lazy objects is not
ported, and a lazy ghost does not complete itself when its last
property is assigned); GH11112Test (the query cache), GH2947Test (the
result cache), GH5562Test, GH6217Test and GH7067Test (the second-level
cache). The tickets with a custom DBAL type in the test file (DDC1998,
DDC2012, DDC2494, DDC2579, DDC2984, DDC3192, DDC3785, DDC5684, GH10288,
GH10747, GH5804, GH5887, GH5988, GH6141, GH8061, GH9335) and the GH
tickets whose models are separate files of the ticket directory
(GH10049, GH10661, GH11017, GH11037, GH11072, GH11149, GH11386,
GH12166, GH12225, GH12505, GH9467) are recorded now (the recorder loads
doctrine/dbal at php2rust-doctrine-dbal's pin, `upstream.tsv`) and not
ported yet. Upstream skips GH10462Test and GH6823Test (MySQL only),
GH10927Test (PostgreSQL only) and `GH11982Test::testSchemaLegacyDbal`
(doctrine/dbal before 4.3): `n/a`.

Locking and inheritance tickets pending: DDC3634Test (its DBAL driver
middleware `LastInsertIdMockMiddleware` is recorded now, but the DBAL
port has no middlewares); GH10288Test (its custom `StringType` subclass
for the enum discriminator is recorded now, its Rust type is not
written yet); DDC1995Test (a `ClassMetadata` as the `INSTANCE OF` parameter
value: `resolveParameterValue()`'s metadata parameter mapping is not
ported; and the query cache).

Queries and hydration, not reproduced and so not claimed:

- The query keeps the parser result of a clean query and caches it in the
  query cache (`Query::setQueryCache()`, `useQueryCache()`,
  `setQueryCacheLifetime()`, `expireQueryCache()`,
  `Configuration::setQueryCache()`): the pool shares the parsed result
  (`cache::pool::PoolValue::ParserResult`) where PHP stores a serialized
  copy, and the cache id hashes the hints' debug form where PHP hashes
  `serialize($this->hints)` (the same hints give the same id). A query is
  dirty after `setHint()` or `setHydrationMode()` (counted on the query
  state) as after `setDQL()`, `setFirstResult()` and `setMaxResults()`.
  `is_a($outputWalkerClass, OutputWalker::class, true)` constructs the
  walker and asks it. The result and hydration caches are not ported:
  they need DBAL's `QueryCacheProfile` and `Connection::executeCacheQuery()`,
  which php2rust-doctrine-dbal does not port; `Configuration` keeps the
  result, hydration and metadata cache pools (set/get only, shared by its
  clones), and the metadata factory does not cache.
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
`NativeQueryTest::testFluentInterface` (the result cache);
Functional `PersistentCollectionTest` is `n/a` (upstream skips it: its
models extend persistence 2's `PersistentObject`); the criteria and extra-lazy
tests of the Ticket, filter, enum and inheritance test files (GH7767,
GH7836, GH7864, SQLFilterTest); the result cache tests
(ResultCacheTest, DBAL's `QueryCacheProfile` is not ported) and the
query and result cache tests of Query QueryTest and `testProcessParameterValueClassMetadata`;
`Functional\QueryTest::testJoinPartialObjectHydration` (partial objects)
and `testQueryBuilderWithStringWhereClauseContainingOrAndConditionalPrimary`
(the query builder); `AbstractHydratorTest`'s `onClear` listener tests;
`CustomHydratorTest` (custom hydration modes);
`LifecycleCallbackTest::testCascadedEntitiesLoadedInPostLoad` (its
`postLoad()` counts an uninitialized collection, which a synchronous
callback body cannot load); the remaining ManyToManyEventTest and
`OneToManyUnidirectionalAssociationTest::testPersistOwningInverseCascade`
tests (not yet translated); and `UnitOfWorkTest::testItPreservesTheOriginalExceptionOnRollbackFailure`
(`Thrown` has no previous exception; a failing rollback's error replaces
the commit's).
