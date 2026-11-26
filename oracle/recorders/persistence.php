<?php
// Records doctrine/persistence answers that its tests leave open, for
// crates/php2rust-doctrine-persistence/tests/persistence_vectors.rs:
// EnumReflectionProperty's setValue() and getValue() over PHP values
// (strict from() on one value, array_map()'s coercive from() on arrays,
// the TypeErrors of its parameter types), the messages of PHP's
// reflection exceptions, and the AssertionError of
// AbstractManagerRegistry::getManager() for a service that is not an
// ObjectManager. Loads the package from upstream/doctrine/persistence.

namespace {
    ini_set('display_errors', 'stderr');
    require __DIR__ . '/lib/typed.php';

    spl_autoload_register(static function (string $class): void {
        $prefix = 'Doctrine\\Persistence\\';
        if (str_starts_with($class, $prefix)) {
            require __DIR__ . '/../../upstream/doctrine/persistence/src/'
                . str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
        }
    });
}

namespace Doctrine\Tests\Persistence\Vectors {
    enum Suit: string
    {
        case Hearts = 'H';
        case Diamonds = 'D';
        case Clubs = '1';
    }

    enum Rank: int
    {
        case One = 1;
        case Two = 2;
    }

    final class Holder
    {
        public Suit|null $suit = null;
        public array|null $suits = null;
        public Rank|null $rank = null;
        public array|null $ranks = null;
        public int $typed;
    }

    final class Registry extends \Doctrine\Persistence\AbstractManagerRegistry
    {
        protected function getService(string $name): object
        {
            return new \stdClass();
        }

        protected function resetService(string $name): void
        {
        }

        public function getAliasNamespace(string $alias): string
        {
            return '';
        }
    }
}

namespace {
    use Doctrine\Persistence\Reflection\EnumReflectionProperty;
    use Doctrine\Tests\Persistence\Vectors\Holder;
    use Doctrine\Tests\Persistence\Vectors\Rank;
    use Doctrine\Tests\Persistence\Vectors\Registry;
    use Doctrine\Tests\Persistence\Vectors\Suit;

    $scalars = ['H', 'D', 'A', '', '1', '2', ' 1', '1.0', '1e0', 0, 1, 2, 3, true, false, 1.0, 1.5, Suit::Hearts, Rank::Two, new stdClass()];
    $arrays = [[], ['H'], ['H', 'D'], ['A'], [1], ['1'], [2], ['2'], [1.0], [true], [null], [Suit::Hearts, 'D'], ['H', Suit::Hearts], [Rank::One, 2], ['a' => 'H', 'b' => 'D'], [[]]];

    $set = [];
    foreach ([['suit', Suit::class, $scalars], ['rank', Rank::class, $scalars], ['suits', Suit::class, $arrays], ['ranks', Rank::class, $arrays]] as [$name, $enum, $values]) {
        foreach ($values as $value) {
            $holder   = new Holder();
            $property = new EnumReflectionProperty(new ReflectionProperty(Holder::class, $name), $enum);
            $set[]    = [
                $name,
                $enum,
                typed_value($value),
                typed_call(static function () use ($property, $holder, $value, $name) {
                    $property->setValue($holder, $value);

                    return $holder->$name;
                }),
                typed_call(static fn () => $property->getValue($holder)),
            ];
        }
    }

    // getValue() of raw values that are not enums.
    $get = [];
    foreach ([['suits', ['x']], ['suits', [Suit::Hearts, 'x']], ['suits', [1]], ['suits', [[]]], ['suits', [Suit::Hearts, Rank::One]], ['ranks', [Rank::Two, null]]] as [$name, $raw]) {
        $holder        = new Holder();
        $holder->$name = $raw;
        $property      = new EnumReflectionProperty(new ReflectionProperty(Holder::class, $name), Suit::class);
        $get[]         = [$name, typed_value($raw), typed_call(static fn () => $property->getValue($holder))];
    }

    $reflection = [
        'class' => typed_call(static fn () => new ReflectionClass('Missing\\Cls')),
        'property' => typed_call(static fn () => new ReflectionProperty(Holder::class, 'nope')),
        'method' => typed_call(static fn () => (new ReflectionClass(Holder::class))->getMethod('nope')),
        'interface' => typed_call(static fn () => (new ReflectionClass(Holder::class))->implementsInterface('Missing')),
        'not_interface' => typed_call(static fn () => (new ReflectionClass(Holder::class))->implementsInterface(Holder::class)),
        'uninitialized' => typed_call(static fn () => (new ReflectionProperty(Holder::class, 'typed'))->getValue(new Holder())),
    ];
    try {
        new ReflectionClass('Missing\\Cls');
    } catch (ReflectionException $e) {
        $reflection['class_code'] = $e->getCode();
    }

    $registry = new Registry('ORM', [], ['default' => 'default'], 'default', 'default');
    try {
        $registry->getManager();
        $assertion = null;
    } catch (Throwable $e) {
        $assertion = [$e::class, $e->getMessage(), $e->getCode()];
    }

    echo json_encode([
        'php' => PHP_VERSION,
        'set' => $set,
        'get' => $get,
        'reflection' => $reflection,
        'assertion' => $assertion,
    ], JSON_PRETTY_PRINT | JSON_THROW_ON_ERROR), "\n";
}
