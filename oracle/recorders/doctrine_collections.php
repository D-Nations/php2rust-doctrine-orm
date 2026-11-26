<?php
// Records doctrine/collections answers that its tests leave open, for
// crates/php2rust-doctrine-collections/tests/collections_vectors.rs:
// ArrayCollection's internal pointer under random call sequences,
// ClosureExpressionVisitor's comparisons over PHP values, matching()'s
// sorting and slicing, slice(), and the messages of the errors on the
// way. Loads the package from upstream/doctrine/collections; its two
// dependencies are stubbed (doctrine/deprecations' Deprecation::trigger()
// as a no-op, symfony/polyfill-php86's SortDirection enum).

namespace Doctrine\Deprecations {
    final class Deprecation
    {
        public static function trigger(string $package, string $link, string $message, mixed ...$args): void
        {
        }
    }
}

namespace {
    if (!enum_exists('SortDirection')) {
        enum SortDirection
        {
            case Ascending;
            case Descending;
        }
    }

    use Doctrine\Common\Collections\ArrayCollection;
    use Doctrine\Common\Collections\Criteria;
    use Doctrine\Common\Collections\Expr\ClosureExpressionVisitor;
    use Doctrine\Common\Collections\Expr\Comparison;
    use Doctrine\Common\Collections\Expr\Value;
    use Doctrine\Common\Collections\Order;

    require __DIR__ . '/lib/typed.php';

    spl_autoload_register(static function (string $class): void {
        $prefix = 'Doctrine\\Common\\Collections\\';
        if (str_starts_with($class, $prefix)) {
            $file = '/work/upstream/doctrine/collections/src/' . str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
            if (is_file($file)) {
                require $file;
            }
        }
    });

    set_error_handler(static fn (): bool => true); // warnings and deprecations: PHP goes on

    mt_srand(486);

    // --- the internal pointer ---------------------------------------------
    $keys = [0, 1, 2, 3, 'a', 'b', 'c'];
    $values = [1, 2, 3, 'x', 'y', null];
    $pointer = [];
    for ($s = 0; $s < 300; $s++) {
        $initial = [];
        $n = mt_rand(0, 5);
        for ($i = 0; $i < $n; $i++) {
            $initial[$keys[mt_rand(0, count($keys) - 1)]] = $values[mt_rand(0, count($values) - 1)];
        }
        $c = new ArrayCollection($initial);
        $steps = [];
        for ($i = 0; $i < 25; $i++) {
            $op = ['next', 'first', 'last', 'remove', 'set', 'add', 'removeElement', 'clear', 'next', 'next', 'matching'][mt_rand(0, $i % 7 === 6 ? 10 : 9)];
            $key = $keys[mt_rand(0, count($keys) - 1)];
            $value = $values[mt_rand(0, count($values) - 1)];
            $result = typed_call(static function () use (&$c, $op, $key, $value) {
                switch ($op) {
                    case 'next': return $c->next();
                    case 'first': return $c->first();
                    case 'last': return $c->last();
                    case 'remove': return $c->remove($key);
                    case 'set': $c->set($key, $value); return null;
                    case 'add': $c->add($value); return null;
                    case 'removeElement': return $c->removeElement($value);
                    case 'clear': $c->clear(); return null;
                    case 'matching': $c = $c->matching(Criteria::create()); return null;
                }
            });
            $steps[] = [$op, typed_value($key), typed_value($value), $result, typed_value($c->key()), typed_value($c->current()), typed_value($c->toArray())];
        }
        $pointer[] = ['initial' => typed_value($initial), 'steps' => $steps];
    }

    // --- comparisons --------------------------------------------------------
    $scalars = [null, true, false, 0, 1, -1, 2, 1.5, 0.0, '0', '1', '', 'abc', 'ab', '1e1', '10', ' 1', '01', 'hello world', [], [1, 2], ['a' => 1], [1, '02'], new ArrayObject([2, 'abc'])];
    $operators = ['=', '<>', '<', '<=', '>', '>=', 'IN', 'NIN', 'CONTAINS', 'MEMBER_OF', 'STARTS_WITH', 'ENDS_WITH'];
    $visitor = new ClosureExpressionVisitor();
    $comparisons = [];
    foreach ($operators as $op) {
        foreach ($scalars as $value) {
            if ($value instanceof ArrayObject && $op !== 'MEMBER_OF') {
                continue;
            }
            $closure = $visitor->walkComparison(new Comparison('foo', $op, $value));
            $answers = [];
            foreach ($scalars as $field) {
                if ($field instanceof ArrayObject && $op !== 'MEMBER_OF') {
                    continue;
                }
                $answers[] = [typed_value($field), typed_call(static fn () => $closure(['foo' => $field]))];
            }
            $comparisons[] = ['op' => $op, 'value' => typed_value($value), 'answers' => $answers];
        }
    }

    // --- field values and their errors --------------------------------------
    $fields = [];
    foreach ([
        [['foo' => ['bar' => 1]], 'foo.bar'],
        [['foo' => 1], 'bar'],
        [['foo' => 1], 'foo.bar'],
        [['foo' => null], 'foo.bar'],
        [['foo' => 'x'], 'foo.bar'],
        [['foo' => [1]], 'foo.0'],
        [['1' => 'one'], '1'],
        [5, 'foo'],
        [null, 'foo'],
        [true, 'foo'],
        [(object) ['foo' => 3], 'foo'],
        [(object) ['foo' => 3], 'bar'],
        [new ArrayObject(['foo' => 1]), 'foo'],
    ] as [$object, $field]) {
        $fields[] = [typed_value($object), $field, typed_call(static fn () => ClosureExpressionVisitor::getObjectFieldValue($object, $field))];
    }

    // --- composite expressions over arrays and objects ------------------------
    $builder = Criteria::expr();
    $composites = [];
    foreach (['and' => $builder->andX($builder->eq('foo', 1)), 'or' => $builder->orX($builder->eq('foo', 1)), 'not' => $builder->not($builder->eq('foo', 1))] as $type => $expr) {
        $closure = $visitor->walkCompositeExpression($expr);
        foreach ([['foo' => 1], (object) ['foo' => 1], (object) ['foo' => 2], 7] as $element) {
            $composites[] = [$type, typed_value($element), typed_call(static fn () => $closure($element))];
        }
    }

    // --- callable criteria (Value) ------------------------------------------
    $callables = [];
    foreach ([null, 5, 'nope', [1], [1, 2], [new stdClass(), 'nope'], new stdClass(), 1.5, true] as $callback) {
        $callables[] = [typed_value($callback), typed_call(static fn () => (new ArrayCollection([0, 1, '', 'a', null, [], [0]]))->matching(new Criteria(new Value($callback)))->toArray())];
    }

    // --- sorting --------------------------------------------------------------
    $pools = [
        'ints' => [-2, -1, 0, 1, 2, 3],
        'floats' => [-1.5, 0.0, 0.5, 1.0, 2.25],
        'strings' => ['', 'a', 'B', 'b', 'aa', 'ab'],
        'numeric strings' => ['1', '2', '10', '1.5', '-1', '01'],
        'ints and numeric strings' => [1, 2, 10, '1', '2', '10', '1.5'],
        'null and bools' => [null, true, false],
        'mixed' => [null, true, false, 0, 1, '0', '1', 'a', '', 1.5, [1]],
    ];
    $sorts = [];
    foreach ($pools as $pool => $members) {
        for ($s = 0; $s < 60; $s++) {
            $elements = [];
            $n = mt_rand(0, 9);
            for ($i = 0; $i < $n; $i++) {
                $elements['k' . $i] = ['foo' => $members[mt_rand(0, count($members) - 1)], 'bar' => $members[mt_rand(0, count($members) - 1)], 'id' => $i];
            }
            $orderings = [];
            foreach (['foo', 'bar'] as $field) {
                if (mt_rand(0, 2) > 0) {
                    $orderings[$field] = [Order::Ascending, Order::Descending, SortDirection::Ascending, SortDirection::Descending][mt_rand(0, 3)];
                }
            }
            if ($orderings === []) {
                $orderings['foo'] = Order::Ascending;
            }
            $named = [];
            foreach ($orderings as $field => $direction) {
                $named[] = [$field, $direction instanceof Order ? 'Order' : 'SortDirection', $direction->name];
            }
            $sorts[] = [
                'pool' => $pool,
                'elements' => typed_value($elements),
                'orderings' => $named,
                'result' => typed_call(static fn () => array_keys((new ArrayCollection($elements))->matching(new Criteria(null, $orderings))->toArray())),
            ];
        }
    }

    // --- slicing --------------------------------------------------------------
    $slices = [];
    $lengths = [null, -3, -1, 0, 1, 2, 5];
    foreach ([[], [1], ['a' => 1, 'b' => 2], [5 => 'x', 6 => 'y', 'z' => 'z', 7 => 'w']] as $elements) {
        foreach ([-5, -2, -1, 0, 1, 2, 3, 5] as $offset) {
            foreach ($lengths as $length) {
                $c = new ArrayCollection($elements);
                $slices[] = [
                    typed_value($elements),
                    $offset,
                    $length,
                    typed_value($c->slice($offset, $length)),
                    typed_value($c->matching(new Criteria(null, null, $offset, $length))->toArray()),
                ];
            }
        }
    }

    // --- messages ---------------------------------------------------------------
    $messages = [
        'Order::from' => typed_call(static fn () => Order::from('asc')),
        'add after PHP_INT_MAX' => typed_call(static function () {
            $c = new ArrayCollection([PHP_INT_MAX => 1]);
            $c->add(2);
        }),
        'toString' => typed_call(static fn () => preg_replace('/@[0-9a-f]{32}$/', '@<hash>', (string) new ArrayCollection())),
    ];

    echo json_encode([
        'pointer' => $pointer,
        'comparisons' => $comparisons,
        'fields' => $fields,
        'composites' => $composites,
        'callables' => $callables,
        'sorts' => $sorts,
        'slices' => $slices,
        'messages' => $messages,
    ], JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES), "\n";
}
