<?php
// Describes the doctrine/orm test models that the mapping tests load, for
// crates/php2rust-doctrine-orm/tests/common/models.rs: each class's kind,
// flags, parent and interfaces, its declared properties (type, default,
// visibility) and methods, an enum's cases, and the mapping attributes of
// the class, its properties and its methods (newInstance()'s public
// properties). The tests declare the classes in persistence's class
// registry and the attributes in the ORM's attribute registry from it,
// as a derive macro would.
//
// Loads the models, and the test files that declare models next to their
// tests, from upstream/doctrine/orm, with stand-ins for PHPUnit and
// doctrine/deprecations (only their names are needed to declare the test
// classes).

namespace {
    ini_set('display_errors', 'stderr');
    error_reporting(E_ALL);
    require __DIR__ . '/lib/typed.php';

    $root = __DIR__ . '/../../upstream/doctrine';

    $psr4 = [
        'Doctrine\\Tests\\' => "$root/orm/tests/Tests/",
        'Doctrine\\ORM\\' => "$root/orm/src/",
        'Doctrine\\Persistence\\' => "$root/persistence/src/",
        'Doctrine\\Common\\Collections\\' => "$root/collections/src/",
        'Doctrine\\Common\\' => "$root/event-manager/src/",
        'Doctrine\\Instantiator\\' => "$root/instantiator/src/",
    ];

    spl_autoload_register(static function (string $class) use ($psr4): void {
        foreach ($psr4 as $prefix => $dir) {
            if (str_starts_with($class, $prefix)) {
                $file = $dir . str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
                if (is_file($file)) {
                    require_once $file;

                    return;
                }
            }
        }
    });
}

// Stand-ins: only the declarations matter.
namespace PHPUnit\Framework {
    abstract class TestCase
    {
    }
}

namespace PHPUnit\Framework\Attributes {
    #[\Attribute(\Attribute::TARGET_ALL | \Attribute::IS_REPEATABLE)]
    final class Group
    {
        public function __construct(string $name)
        {
        }
    }
}

namespace Doctrine\Deprecations\PHPUnit {
    trait VerifyDeprecations
    {
    }
}

namespace Doctrine\Deprecations {
    final class Deprecation
    {
        public static function trigger(string $package, string $link, string $message, mixed ...$args): void
        {
        }

        public static function triggerIfCalledFromOutside(string $package, string $link, string $message, mixed ...$args): void
        {
        }
    }
}

namespace Doctrine\DBAL\Types {
    final class Types
    {
        public const ASCII_STRING = 'ascii_string';
        public const BIGINT = 'bigint';
        public const BINARY = 'binary';
        public const BLOB = 'blob';
        public const BOOLEAN = 'boolean';
        public const DATE_MUTABLE = 'date';
        public const DATE_IMMUTABLE = 'date_immutable';
        public const DATEINTERVAL = 'dateinterval';
        public const DATETIME_MUTABLE = 'datetime';
        public const DATETIME_IMMUTABLE = 'datetime_immutable';
        public const DATETIMETZ_MUTABLE = 'datetimetz';
        public const DATETIMETZ_IMMUTABLE = 'datetimetz_immutable';
        public const DECIMAL = 'decimal';
        public const NUMBER = 'number';
        public const FLOAT = 'float';
        public const ENUM = 'enum';
        public const GUID = 'guid';
        public const INTEGER = 'integer';
        public const JSON = 'json';
        public const JSONB = 'jsonb';
        public const SIMPLE_ARRAY = 'simple_array';
        public const SMALLFLOAT = 'smallfloat';
        public const SMALLINT = 'smallint';
        public const STRING = 'string';
        public const TEXT = 'text';
        public const TIME_MUTABLE = 'time';
        public const TIME_IMMUTABLE = 'time_immutable';
    }
}

namespace {
    // symfony/polyfill-php86's enum, which the ORM requires.
    if (! enum_exists('SortDirection', false)) {
        enum SortDirection
        {
            case Ascending;
            case Descending;
        }
    }
}

namespace {
    use Doctrine\ORM\Mapping\MappingAttribute;

    /** Mapping attributes as [class, typed public properties]. */
    function attributes(array $reflectionAttributes): array
    {
        $out = [];
        foreach ($reflectionAttributes as $attribute) {
            $name = $attribute->getName();
            if (! is_subclass_of($name, MappingAttribute::class)) {
                continue;
            }

            $instance = $attribute->newInstance();
            if (str_starts_with($name, 'Doctrine\\ORM\\Mapping\\')) {
                $out[] = typed_value($instance);
                continue;
            }

            // A mapping attribute the application declares.
            $flags = (new ReflectionClass($name))->getAttributes(Attribute::class)[0]->newInstance()->flags;
            $out[] = ['custom', $name, ($flags & Attribute::IS_REPEATABLE) > 0, typed_value($instance)];
        }

        return $out;
    }

    function type_of(?ReflectionType $type): ?array
    {
        if ($type === null) {
            return null;
        }

        $name = $type instanceof ReflectionNamedType ? $type->getName() : (string) $type;

        return [$name, $type->allowsNull()];
    }

    function describe(ReflectionClass $class): array
    {
        $kind = match (true) {
            $class->isEnum() => 'enum',
            $class->isInterface() => 'interface',
            $class->isTrait() => 'trait',
            default => 'class',
        };

        $properties = [];
        $defaults   = $class->getDefaultProperties();
        foreach ($class->getProperties() as $property) {
            if ($property->class !== $class->name || $kind === 'enum') {
                continue;
            }

            $properties[] = [
                'name' => $property->name,
                'visibility' => $property->isPrivate() ? 'private' : ($property->isProtected() ? 'protected' : 'public'),
                'static' => $property->isStatic(),
                'readonly' => $property->isReadOnly(),
                'type' => type_of($property->getType()),
                'default' => $property->hasDefaultValue()
                    ? typed_value($property->isStatic() ? $property->getValue() : $defaults[$property->name] ?? null)
                    : null,
                'attributes' => attributes($property->getAttributes()),
            ];
        }

        $methods = [];
        foreach ($class->getMethods() as $method) {
            if ($method->class !== $class->name) {
                continue;
            }

            $methods[] = [
                'name' => $method->name,
                'visibility' => $method->isPrivate() ? 'private' : ($method->isProtected() ? 'protected' : 'public'),
                'static' => $method->isStatic(),
                'required' => $method->getNumberOfRequiredParameters(),
                'attributes' => attributes($method->getAttributes()),
            ];
        }

        $parent = $class->getParentClass();
        $own    = $class->getInterfaceNames();
        if ($parent !== false) {
            $own = array_values(array_diff($own, $parent->getInterfaceNames()));
        }

        return [
            'name' => $class->name,
            'kind' => $kind,
            'abstract' => $kind === 'class' && $class->isAbstract(),
            'final' => $class->isFinal(),
            'parent' => $parent === false ? null : $parent->name,
            'interfaces' => $own,
            'properties' => $properties,
            'methods' => $methods,
            'cases' => $kind === 'enum' ? array_map(typed_value(...), $class->name::cases()) : [],
            'attributes' => attributes($class->getAttributes()),
        ];
    }

    /** The class names among the strings of a typed value. */
    function class_strings(mixed $typed, array &$out): void
    {
        if (! is_array($typed)) {
            return;
        }

        if (($typed[0] ?? null) === 'string' && count($typed) === 2 && is_string($typed[1])) {
            $string = base64_decode($typed[1]);
            if (preg_match('/^\\\\?[A-Za-z_][A-Za-z0-9_\\\\]*$/', $string) === 1) {
                $out[] = ltrim($string, '\\');
            }

            return;
        }

        foreach ($typed as $item) {
            class_strings($item, $out);
        }
    }

    function load(string $name): bool
    {
        return class_exists($name) || interface_exists($name) || enum_exists($name) || trait_exists($name);
    }

    // The mapping tests whose models (imported or declared next to the
    // tests) are described; arguments replace the list.
    $files = array_slice($argv, 1) ?: [
        'ORM/Mapping/AttributeDriverTest.php',
        'ORM/Mapping/AttributeReaderTest.php',
        'ORM/Mapping/AnsiQuoteStrategyTest.php',
        'ORM/Mapping/BasicInheritanceMappingTest.php',
        'ORM/Mapping/ClassMetadataBuilderTest.php',
        'ORM/Mapping/ClassMetadataFactoryTest.php',
        'ORM/Mapping/ClassMetadataLoadEventTest.php',
        'ORM/Mapping/ClassMetadataTest.php',
        'ORM/Mapping/DefaultQuoteStrategyTest.php',
        'ORM/Mapping/MappingDriverTestCase.php',
        'ORM/Mapping/QuoteStrategyTest.php',
        'ORM/Mapping/ReflectionEmbeddedPropertyTest.php',
        'ORM/Mapping/ReflectionReadonlyPropertyTest.php',
        'ORM/Mapping/PropertyAccessors/EnumPropertyAccessorTest.php',
        'ORM/Mapping/PropertyAccessors/ObjectCastPropertyAccessorTest.php',
        'ORM/Mapping/PropertyAccessors/ReadOnlyAccessorTest.php',
        'ORM/Mapping/PropertyAccessors/TypedNoDefaultPropertyAccessorTest.php',
        'ORM/Mapping/TypedFieldMapperTest.php',
        'ORM/Mapping/TypedEnumFieldMapperTest.php',
        'ORM/UnitOfWorkTest.php',
        'ORM/Functional/LifecycleCallbackTest.php',
        'ORM/Functional/OneToManyBidirectionalAssociationTest.php',
        'ORM/Functional/OneToManyUnidirectionalAssociationTest.php',
        'ORM/Functional/OneToManySelfReferentialAssociationTest.php',
        'ORM/Functional/OneToManyOrphanRemovalTest.php',
        'ORM/Functional/ManyToManyBasicAssociationTest.php',
        'ORM/Functional/ManyToManyBidirectionalAssociationTest.php',
        'ORM/Functional/ManyToManyUnidirectionalAssociationTest.php',
        'ORM/Functional/ManyToManySelfReferentialAssociationTest.php',
        'ORM/Hydration/AbstractHydratorTest.php',
        'ORM/Hydration/ArrayHydratorTest.php',
        'ORM/Hydration/ObjectHydratorTest.php',
        'ORM/Hydration/ResultSetMappingTest.php',
        'ORM/Hydration/ScalarColumnHydratorTest.php',
        'ORM/Hydration/ScalarHydratorTest.php',
        'ORM/Hydration/SimpleObjectHydratorTest.php',
        'ORM/Hydration/SingleScalarHydratorTest.php',
        'ORM/Query/QueryTest.php',
        'ORM/Functional/QueryTest.php',
        'ORM/Functional/BasicFunctionalTest.php',
        'ORM/Functional/EntityRepositoryTest.php',
        'ORM/Functional/PaginationTest.php',
        'Doctrine\\Tests\\DbalTypes\\CustomIdObject',
        'ORM/Functional/EntityRepositoryCriteriaTest.php',
        'ORM/Functional/ExtraLazyCollectionTest.php',
        'ORM/Functional/PersistentCollectionCriteriaTest.php',
        'ORM/LazyCriteriaCollectionTest.php',
        'ORM/PersistentCollectionTest.php',
        'ORM/Functional/NativeQueryTest.php',
        'ORM/Functional/ValueConversionType/OneToManyExtraLazyTest.php',
        'ORM/Functional/ValueConversionType/ManyToManyExtraLazyTest.php',
        'ORM/Functional/ValueConversionType/OneToManyCriteriaMatchingTest.php',
        'ORM/Functional/ValueConversionType/ManyToManyCriteriaMatchingTest.php',
        'ORM/Functional/QueryBuilderParenthesisTest.php',
        'ORM/Functional/IndexByAssociationTest.php',
        'ORM/Functional/TypeTest.php',
        'ORM/Functional/EnumTest.php',
        'ORM/Functional/TypeValueSqlTest.php',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\Order',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\User',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilterAndIndexedRelation\\Company',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilterAndIndexedRelation\\Category',
        'Models/Global/GlobalNamespaceModel.php',
        'ORM/Functional/SingleTableInheritanceTest.php',
        'ORM/Functional/ClassTableInheritanceTest.php',
        'ORM/Functional/ClassTableInheritanceSecondTest.php',
        'ORM/Functional/MappedSuperclassTest.php',
        'ORM/Functional/OneToOneSingleTableInheritanceTest.php',
        'ORM/Functional/OrderedJoinedTableInheritanceCollectionTest.php',
        'ORM/Functional/SchemaTool/CompanySchemaTest.php',
        'ORM/Functional/JoinedTableCompositeKeyTest.php',
        'ORM/Functional/SingleTableCompositeKeyTest.php',
        'ORM/Functional/Locking/OptimisticTest.php',
        'ORM/Functional/QueryDqlFunctionTest.php',
        'ORM/Functional/Ticket/DDC1655Test.php',
        'ORM/Functional/Ticket/DDC168Test.php',
        'ORM/Functional/Ticket/DDC6303Test.php',
        'ORM/Functional/Ticket/DDC832Test.php',
        'ORM/Functional/Ticket/GH11500Test.php',
        'ORM/Functional/Ticket/GH11501Test.php',
        'ORM/Tools/SchemaToolTest.php',
        'ORM/Functional/Ticket/DDC117Test.php',
        'ORM/Functional/ReferenceProxyTest.php',
        'ORM/Functional/ValueObjectsTest.php',
    ];
    $before = array_merge(get_declared_classes(), get_declared_interfaces(), get_declared_traits());
    foreach ($files as $file) {
        $path = "$root/orm/tests/Tests/$file";
        if (! str_ends_with($file, '.php')) {
            load($file);
            continue;
        }

        // The models a test file imports.
        preg_match_all('/^use (Doctrine\\\\Tests\\\\(?:Models|ORM\\\\Mapping\\\\Fixtures)\\\\[^ ;]+)(?: as [^;]+)?;/m', file_get_contents($path), $uses);
        foreach ($uses[1] as $use) {
            load($use);
        }

        require_once $path;
    }

    $isWanted = static function (string $name): bool {
        if (! str_starts_with($name, 'Doctrine\\Tests\\') && str_contains($name, '\\')) {
            return false;
        }

        $class = new ReflectionClass($name);

        return ! $class->isAnonymous()
            && ! $class->isInternal()
            && ! is_subclass_of($name, PHPUnit\Framework\TestCase::class);
    };

    // The classes the attributes name (target entities, repositories,
    // listeners, discriminator maps), until no new one appears.
    $described = [];
    do {
        $declared = array_merge(get_declared_classes(), get_declared_interfaces(), get_declared_traits());
        $new      = array_filter(array_diff($declared, $before, array_keys($described)), $isWanted);
        foreach ($new as $name) {
            $described[$name] = describe(new ReflectionClass($name));
            $strings          = [];
            class_strings($described[$name], $strings);
            $namespace = (new ReflectionClass($name))->getNamespaceName();
            foreach ($strings as $string) {
                if (str_starts_with($string, 'Doctrine\\Tests\\')) {
                    load($string);
                } elseif (! str_contains($string, '\\') && $namespace !== '' && preg_match('/^[A-Z]/', $string) === 1) {
                    // A class name relative to the class's namespace.
                    load($namespace . '\\' . $string);
                }
            }
        }
    } while ($new !== []);

    // Parents and interfaces before the classes that use them.
    $ordered = [];
    $visit   = static function (string $name) use (&$visit, &$ordered, $described): void {
        if (isset($ordered[$name]) || ! isset($described[$name])) {
            return;
        }

        $class = new ReflectionClass($name);
        if ($class->getParentClass() !== false) {
            $visit($class->getParentClass()->name);
        }

        foreach ($class->getInterfaceNames() as $interface) {
            $visit($interface);
        }

        $ordered[$name] = $described[$name];
    };
    $names = array_keys($described);
    sort($names);
    foreach ($names as $name) {
        $visit($name);
    }

    $lines = array_map(
        static fn (array $class): string => json_encode($class, JSON_UNESCAPED_SLASHES | JSON_THROW_ON_ERROR),
        array_values($ordered),
    );
    echo '{"php": ', json_encode(PHP_VERSION), ', "classes": [', "\n", implode(",\n", $lines), "\n]}\n";
}
