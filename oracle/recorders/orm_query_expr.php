<?php
// Records what doctrine/orm's DQL expression builder (Doctrine\ORM\Query\Expr
// and the Expr\* classes) gives for expressions its tests leave open, for
// crates/php2rust-doctrine-orm/tests/query_expr_vectors.rs: composites of
// strings, comparisons and nested composites (DDC-1237's parentheses),
// functions and math over every kind of PHP value, quoting of literals and
// IN() lists, joins, froms, order-bys with their deprecations, and the
// exceptions. Each case is a spec the Rust test builds the same way:
//
//   ["v", typed]                       a PHP value (lib/typed.php)
//   ["call", method, [spec, ...]]      $expr->method(...)
//   ["new", class, [spec, ...]]        new Expr\Class(...)
//   ["add", spec, [spec, ...]]         $object->add(...) for each, then $object
//   ["orderAdd", spec, [spec, ...]]    OrderBy::add($sort, ...$order)
//
// with the result of (string) on what the spec gives (or the exception)
// and the number of deprecations it triggered. Loads the package from
// upstream/doctrine/orm; SortDirection is symfony/polyfill-php86's enum,
// Deprecation::trigger() a counter.

namespace {
    ini_set('display_errors', 'stderr');
    require __DIR__ . '/lib/typed.php';

    if (!enum_exists('SortDirection', false)) {
        enum SortDirection
        {
            case Ascending;
            case Descending;
        }
    }

    $src = __DIR__ . '/../../upstream/doctrine/orm/src';
    require "$src/Internal/NoUnknownNamedArguments.php";
    require "$src/Query/Expr.php";
    foreach (['Base', 'Composite', 'Andx', 'Orx', 'Comparison', 'Func', 'Join', 'Select', 'From', 'OrderBy', 'GroupBy', 'Literal', 'Math'] as $class) {
        require "$src/Query/Expr/$class.php";
    }
}

namespace Doctrine\Deprecations {
    final class Deprecation
    {
        public static int $count = 0;

        public static function trigger(string $package, string $link, string $message, mixed ...$args): void
        {
            self::$count++;
        }
    }
}

namespace Doctrine\Tests\ORM\Query\Vectors {
    use Doctrine\Deprecations\Deprecation;
    use Doctrine\ORM\Query\Expr;
    use SortDirection;

    function v(mixed $value): array
    {
        return ['v', typed_value($value)];
    }

    function call(string $method, array ...$args): array
    {
        return ['call', $method, $args];
    }

    function make(string $class, array ...$args): array
    {
        return ['new', $class, $args];
    }

    function add(array $target, array ...$args): array
    {
        return ['add', $target, $args];
    }

    function build(array $spec): mixed
    {
        static $expr = new Expr();

        return match ($spec[0]) {
            'v' => decode($spec[1]),
            'dir' => constant(SortDirection::class . '::' . $spec[1]),
            'call' => $expr->{$spec[1]}(...array_map(build(...), $spec[2])),
            'new' => new ('Doctrine\\ORM\\Query\\Expr\\' . $spec[1])(...array_map(build(...), $spec[2])),
            'add' => (static function () use ($spec) {
                $target = build($spec[1]);
                foreach ($spec[2] as $arg) {
                    $target->add(build($arg));
                }

                return $target;
            })(),
            'orderAdd' => (static function () use ($spec) {
                $target = build($spec[1]);
                $target->add(...array_map(build(...), $spec[2]));

                return $target;
            })(),
        };
    }

    /** The inverse of typed_value() for the kinds the specs use. */
    function decode(array $typed): mixed
    {
        return match ($typed[0]) {
            'null' => null,
            'bool', 'int' => $typed[1],
            'float' => unpack('E', hex2bin($typed[1]))[1],
            'string' => base64_decode($typed[1]),
            'array' => array_combine(
                array_map(static fn ($p) => decode($p[0]), $typed[1]),
                array_map(static fn ($p) => decode($p[1]), $typed[1]),
            ) ?: [],
            'object' => (object) decode(['array', $typed[2]]),
        };
    }

    $strings = ['u.id', 'u.name', 'a OR b', 'x AND y', 'xory', "a\tor\tb", "a\nAND\nb", "a\vor\x0Cb", 'OR b', 'a or', "it's", "''", '', '0', ':p', '?1', 'ü'];
    $scalars = [0, 5, -3, PHP_INT_MAX, 1.5, 0.1 + 0.2, 1e100, -0.0, 1.0, 1e15, 1e-7, true, false, null];
    $others = [[1, 2], ['k' => 'v'], [[1, 2], 3], (object) ['a' => 1]];

    $cases = [];

    // Values in every position that takes mixed.
    foreach ([...$strings, ...$scalars, ...$others] as $value) {
        $cases[] = call('eq', v($value), v('x'));
        $cases[] = call('neq', v('x'), v($value));
        $cases[] = call('avg', v($value));
        $cases[] = call('sum', v($value), v(2));
        $cases[] = call('concat', v($value), v('y'));
        $cases[] = call('trim', v($value));
        $cases[] = call('countDistinct', v($value), v('z'));
        $cases[] = call('substring', v($value), v(1));
        $cases[] = make('Func', v('F'), v($value));
        $cases[] = make('Andx', v($value));
        $cases[] = make('Orx', v([$value, 'b = 1']));
        $cases[] = make('Select', v($value));
        $cases[] = make('GroupBy', v([$value, 'g']));
        $cases[] = make('Literal', v($value));
        $cases[] = make('Composite', v(['c = 1', $value]));
        $cases[] = add(call('andX'), v($value));
        $cases[] = add(call('orX', v('o = 1')), v($value));
        if (is_bool($value) || is_int($value) || is_float($value) || is_string($value)) {
            $cases[] = call('literal', v($value));
            $cases[] = call('in', v('u.id'), v([$value, 'x']));
            $cases[] = call('notIn', v('u.id'), v($value));
        } else {
            $cases[] = call('in', v('u.id'), v([$value]));
        }
    }

    // Composites of composites.
    $leaf = [
        v('a = 1'), v('b = 2 OR c = 3'), v('d AND e'), call('eq', v('f'), v(1)), call('lt', v('g'), v('h OR i')),
        call('andX'), call('orX'), call('andX', v('j = 1')), call('orX', v('k = 1'), v('l = 2')),
        call('andX', v('m = 1'), call('orX', v('n = 1'), v('o = 1'))), call('count', v('u')),
        call('not', v('p AND q')), call('prod', v(1), v(2)), make('Composite', v(['r', 's'])),
        make('Comparison', call('andX', v('t'), v('u')), v('='), v(1)), v(null),
    ];
    mt_srand(7);
    for ($i = 0; $i < 300; $i++) {
        $parts = [];
        for ($n = mt_rand(0, 4); $n > 0; $n--) {
            $part = $leaf[mt_rand(0, count($leaf) - 1)];
            if (mt_rand(0, 3) === 0) {
                $part = call(mt_rand(0, 1) ? 'andX' : 'orX', $part, $leaf[mt_rand(0, count($leaf) - 1)]);
            }
            $parts[] = $part;
        }
        $cases[] = mt_rand(0, 2) === 0
            ? add(call(mt_rand(0, 1) ? 'andX' : 'orX'), ...$parts)
            : call(mt_rand(0, 1) ? 'andX' : 'orX', ...$parts);
    }

    // Math and functions nested.
    $cases[] = call('quot', call('diff', v(100), v(20)), call('sum', v(1), call('prod', v(2), v(3))));
    $cases[] = call('diff', call('avg', v('x')), call('quot', v('a'), v('b')));
    $cases[] = call('eq', call('sum', v(1), v(2)), call('abs', call('diff', v(1), v(2))));
    $cases[] = call('mod', call('length', v('s')), v(2));
    $cases[] = call('exists', call('andX', v('a'), v('b')));
    $cases[] = make('Math', v([1]), v('%'), v((object) []));
    $cases[] = call('in', v('u.id'), v(['a' => 1, 'b' => "x'y", 7 => 2.5]));
    $cases[] = call('in', v('u.id'), call('literal', v('lit')));
    $cases[] = call('in', v('u.id'), v('scalar'));
    $cases[] = call('in', v('u.id'), v([]));
    $cases[] = call('notIn', v('u.id'), v([true, false, -0.0, '']));
    $cases[] = call('between', v('u.id'), v('1'), v(9));
    $cases[] = call('between', v(1.5), v(1), v(2));
    $cases[] = call('between', v([1]), v(1), v(2));
    $cases[] = call('isNull', v(''));
    $cases[] = call('isNotNull', v('u'));
    $cases[] = call('like', v('u'), v(null));
    $cases[] = call('isMemberOf', v(''), v(''));
    $cases[] = call('countDistinct');
    $cases[] = call('concat');
    $cases[] = add(make('Select', v(['s'])), call('count', v('x')), call('eq', v(1), v(1)));
    $cases[] = add(make('GroupBy'), v('g1'), call('max', v('x')));

    // (array) of an expression object: all of its properties.
    foreach ([call('eq', v('a'), v(1)), call('andX', v('b'), v('c')), call('max', v('m')), call('sum', v(1), v(2)),
              make('From', v('F'), v('f')), make('Join', v('left'), v('j'), v('a'), v('ON'), v('x'), v('i')),
              call('desc', v('d')), call('literal', v('l'))] as $object) {
        $cases[] = make('Andx', $object);
        $cases[] = make('Func', v('F'), $object);
        $cases[] = call('trim', $object);
        $cases[] = call('literal', $object);
        $cases[] = call('andX', $object, v('z'));
    }

    // Joins and froms.
    foreach ([null, '', '0', 'a'] as $alias) {
        foreach ([null, '', '0', 'i.id'] as $indexBy) {
            $cases[] = make('From', v('Foo'), v('f'), v($indexBy));
            foreach ([null, 'on', 'WITH'] as $conditionType) {
                foreach ([v(null), v(''), v('0'), v('x = 1'), call('eq', v('a'), v('b')), call('andX', v('c'), v('d')), call('andX')] as $condition) {
                    $cases[] = make('Join', v('left'), v('f.bar'), v($alias), v($conditionType), $condition, v($indexBy));
                }
            }
        }
    }
    $cases[] = make('Join', v('inner'), v('j'));

    // Order-bys.
    foreach ([['dir', 'Ascending'], ['dir', 'Descending'], v('desc'), v(''), v('0'), v(null)] as $order) {
        foreach (['u.n', '', '0'] as $sort) {
            $cases[] = make('OrderBy', v($sort), $order);
            $cases[] = ['orderAdd', make('OrderBy', v('a'), ['dir', 'Descending']), [v($sort), $order]];
            $cases[] = ['orderAdd', make('OrderBy', v('a'), ['dir', 'Descending']), [v($sort)]];
        }
    }
    $cases[] = make('OrderBy');
    $cases[] = make('OrderBy', v(null));
    $cases[] = call('asc', v('u'));
    $cases[] = call('desc', v(''));

    $out = [];
    foreach ($cases as $spec) {
        $before = Deprecation::$count;
        $result = typed_call(static fn () => (string) build($spec));
        $out[] = ['spec' => $spec, 'result' => $result, 'deprecations' => Deprecation::$count - $before];
    }

    // One case per line.
    echo "[\n", implode(",\n", array_map(
        static fn (array $case) => json_encode($case, JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR),
        $out,
    )), "\n]\n";
}
