# Doctrine Collections

Private workspace crate: a port of `doctrine/collections` 3.1.0
(`7c2f25ed`), as installed by symfony/demo. MIT; Doctrine source and tests
copyright the Doctrine Project. Unofficial, not affiliated with or
endorsed by Doctrine.

```rust
use php2rust_doctrine_collections::{ArrayCollection, Criteria, Order};
use php_value::{Value, php_array};

let collection = ArrayCollection::new(php_array![
    "a" => php_array!["name" => "b", "rank" => 2],
    "b" => php_array!["name" => "a", "rank" => 1],
    "c" => php_array!["name" => "c", "rank" => 3],
]);

let mut criteria = Criteria::create();
criteria.r#where(Criteria::expr().gt("rank", 1));
criteria.order_by([("name", Order::Ascending)]);

let matched = collection.matching(&criteria).unwrap();
assert_eq!(matched.get_keys(), php_array!["a", "c"]);
assert_eq!(matched.count(), 2);
assert!(matched.contains_key(&"c".into()));
assert_eq!(
    collection.get(&"b".into()),
    Value::Array(php_array!["name" => "a", "rank" => 1])
);
```

Elements are `php_value::Value`s under `php_value::Key`s, in PHP's order
and with PHP's key rules (`PhpArray`). PHP closures are Rust closures over
those values; what PHP throws is a `php_value::Thrown` (class and
message). The interfaces are traits (`ReadableCollection`, `Collection`,
`Selectable`) over `Box<dyn Collection>`; their methods return `Result`
and take `&mut self`, as a lazy collection initializes itself (and may
throw) on any call. `ArrayCollection` also has the same methods directly,
without `Result` where it cannot fail. The example is the crate's doctest.

| Contract | Source | Evidence |
|---|---|---|
| `ArrayCollection`: `toArray()`, `first()`/`last()`/`key()`/`current()`/`next()` with PHP's internal pointer (`false` and `null` past the elements), `remove()`, `removeElement()`, `containsKey()`, `contains()` and `indexOf()` (`===`), `get()`, `getKeys()`, `getValues()`, `count()`, `set()`, `add()` (the `Error` after `PHP_INT_MAX`), `isEmpty()`, `getIterator()`, `map()`, `reduce()`, `filter()` (value and key), `findFirst()`, `exists()`, `forAll()`, `partition()`, `clear()`, `slice()` (`array_slice()` keeping keys), `ArrayAccess`, `__toString()` (`class@spl_object_hash`), `createFrom()` for extending classes | `ArrayCollection.php` | ArrayCollectionTest (25), CollectionTest (34), DerivedCollectionTest (1); oracle vectors: 300 random call sequences of the pointer methods and changes (`tests/collections_vectors.rs`), 224 `slice()` and `matching()` slices |
| `matching()`: filter by the criteria's expression, stable sort by its orderings (`Order` or `\SortDirection`, later fields breaking ties), slice by first and max results when either is positive, keys kept; a callable `Value` criteria as `array_filter()`'s callback (`null`, `\Closure`, `__invoke()`, `[$object, 'method']`) with PHP's `TypeError` messages for the others | `ArrayCollection.php` | the matching tests of ArrayCollectionTest, CollectionTest and AbstractLazyCollectionTest; 420 recorded sorts over seven value pools, 9 recorded callbacks |
| `ReadableCollection`, `Collection`, `Selectable` | `ReadableCollection.php`, `Collection.php`, `Selectable.php` | the tests above, through `Box<dyn Collection>` |
| `AbstractLazyCollection`: `doInitialize()` once on first use, `isInitialized()`, the `LogicException` when it set no collection, every method delegated | `AbstractLazyCollection.php` | AbstractLazyCollectionTest (37), AbstractLazyArrayCollectionTest (25), over the `LazyArrayCollection` fixture |
| `Criteria`: `create()`, `expr()`, the constructor, `where()`/`andWhere()`/`orWhere()` nesting, `getOrderings()` (as `\SortDirection`) and the deprecated `orderings()` (as `Order`), `orderBy()`, first and max results; the deprecated `accessRawFieldValues` arguments trigger `https://github.com/doctrine/collections/pull/486` | `Criteria.php`, `Order.php` | CriteriaTest (14) |
| `Comparison`, `CompositeExpression` (its `RuntimeException`s), `Value`, `ExpressionVisitor` (`dispatch()`, `visit()`), `ExpressionBuilder` | `Expr/Comparison.php`, `Expr/CompositeExpression.php`, `Expr/Value.php`, `Expr/Expression.php`, `Expr/ExpressionVisitor.php`, `ExpressionBuilder.php` | ComparisonTest (1), CompositeExpressionTest (4), ValueTest (2), ExpressionBuilderTest (16 of 17) |
| `ClosureExpressionVisitor`: `getObjectFieldValue()` (dotted paths, array keys, raw property values up the parent classes, its `RuntimeException` and `TypeError`), `sortByField()`, the closures of the twelve operators (`===`, PHP 8's `<`/`<=`/`>`/`>=`, `in_array()` strict for scalars, `str_contains()`/`str_starts_with()`/`str_ends_with()` on `(string)` values, `MEMBER_OF` over arrays and `\Traversable`s), `AND`/`OR`/`NOT` closures (objects only, as upstream types them), the unknown operator and composite messages | `Expr/ClosureExpressionVisitor.php` | ClosureExpressionVisitorTest (34); 6,395 recorded comparisons over 23 PHP values per side (and an `ArrayObject` for `MEMBER_OF`), 13 recorded field reads, 12 recorded composite calls |

`reflection` is the part of PHP's Reflection API that
`getObjectFieldValue()` uses: a class with non-public or hooked properties
registers once (`register_raw_properties::<T>()`, trait `RawProperties`)
to have them read raw; any other object is read through its public
properties (`PhpObject::properties()`). `deprecations` is the part of
doctrine/deprecations the tests observe, as in php2rust-doctrine-dbal.
`SortDirection` is PHP 8.6's enum, which the package takes from
symfony/polyfill-php86.

Vectors: `scripts/record.sh collections` runs
`oracle/recorders/doctrine_collections.php` (the package from
`upstream/doctrine/collections`, its two dependencies stubbed) and writes
`tests/data/doctrine_collections.json`.

## Differences

- `PhpArray` has no internal pointer or internal layout. A new
  collection's pointer is at its first element (PHP copies the pointer of
  the array passed in). When the pointer is past the end and elements are
  added, PHP's pointer may reach them or not depending on the array's
  layout (packed or hash, holes); the port's reaches the first one added.
  In the recorded sequences this changed 35 of 1,837 later answers; the
  vector test does not compare the pointer from there until `first()`,
  `last()` or `clear()`.
- Sorting is a stable merge sort. For field values that compare
  consistently it gives PHP's order; where two values are loosely equal
  but not identical (`1` and `'1'`, `null` and `false`), `sortByField()`
  answers `-1` both ways and the order depends on the algorithm's
  comparisons, which are not PHP's `zend_sort()`. The vector test checks
  the order for ints, floats, strings and numeric strings, and only the
  elements for the mixed pools.
- `TypeError` messages of user functions and closures
  (`getObjectFieldValue()`, the composite closures) stop before PHP's `,
  called in <file> on line <line>`.
- `getObjectFieldValue()` reads an unregistered object's public
  properties; it cannot see an uninitialized typed property (PHP throws
  an `Error` for it, the port a `RuntimeException` that the field does
  not exist) unless the class registers and says so.
- A callable `Value` criteria that names a function or a class
  (`'is_numeric'`, `['Class', 'method']`) is not resolved: the port gives
  PHP's message for an unknown function or class.
- Statically typed arguments: `andX()` with a non-`Expression`
  (ExpressionBuilderTest `testInvalidAndXArgument`, n/a) and a string
  among `CompositeExpression`'s children (a row of `testExceptions`'
  provider) cannot be written. Orderings keep a numeric field name as a
  string (PHP makes it an int key, which `matching()` then rejects).
- `__toString()`'s handle is a process-wide counter, never reused (PHP
  reuses freed object handles). `orderings()` is `#[deprecated]` where PHP
  marks it `#[Deprecated]`.
- `ArrayCollectionTest::testUnserializeEmptyArrayCollection` checks PHP's
  `serialize()`/`unserialize()` of an extending class with
  `__serialize()`/`__unserialize()`; the port serializes `toArray()` with
  `php_value::serialize` and rebuilds a default `ArrayCollection` with
  `set()`.
