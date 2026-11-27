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
        // The DBAL types the custom types of the test files extend.
        'Doctrine\\DBAL\\' => "$root/dbal/src/",
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
        'ORM/Functional/OneToOneEagerLoadingTest.php',
        'ORM/Functional/ReadOnlyTest.php',
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
        'Doctrine\\Tests\\Models\\Generic\\NonAlphaColumnsEntity',
        'ORM/Functional/SchemaTool/DBAL483Test.php',
        'ORM/Functional/OneToOneSelfReferentialAssociationTest.php',
        'ORM/Functional/CustomForeignKeyNameTest.php',
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
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\Practice',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\Patient',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\PatientInsurance',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\PrimaryPatInsurance',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\SecondaryPatInsurance',
        'Doctrine\\Tests\\ORM\\Functional\\Ticket\\SwitchContextWithFilter\\Entity\\Insurance',
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
        'ORM/Functional/SchemaTool/DBAL483Test.php',
        'Models/DDC3699/DDC3699Parent.php',
        'Models/DDC3699/DDC3699RelationOne.php',
        'Models/DDC3699/DDC3699RelationMany.php',
        'Models/DDC3699/DDC3699Child.php',
        'Models/CompositeKeyRelations/InvoiceClass.php',
        'Models/CompositeKeyRelations/CustomerClass.php',
        'Models/GeoNames/City.php',
        'Models/VersionedManyToOne/Category.php',
        'Models/VersionedManyToOne/Article.php',
        'ORM/Mapping/XmlMappingDriverTest.php',
        'Models/InvalidXml.php',
        'Models/Project/Project.php',
        'Models/Project/ProjectId.php',
        'Models/Project/ProjectInvalidMapping.php',
        'Models/Project/ProjectName.php',
        'Models/ValueObjects/EmbeddableBase.php',
        'Models/ValueObjects/EmbeddableSubclass.php',
        'Models/ValueObjects/Name.php',
        'Models/ValueObjects/Person.php',
        'Models/DDC3293/DDC3293Address.php',
        'Models/DDC3293/DDC3293User.php',
        'Models/DDC3293/DDC3293UserPrefixed.php',
        'Models/GH7141/GH7141Article.php',
        'Models/GH7316/GH7316Article.php',
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
        'ORM/Functional/ValueConversionType/ManyToManyCompositeIdForeignKeyTest.php',
        'ORM/Functional/ValueConversionType/ManyToManyCompositeIdTest.php',
        'ORM/Functional/ValueConversionType/ManyToManyTest.php',
        'ORM/Functional/ValueConversionType/OneToManyCompositeIdForeignKeyTest.php',
        'ORM/Functional/ValueConversionType/OneToManyCompositeIdTest.php',
        'ORM/Functional/ValueConversionType/OneToManyTest.php',
        'ORM/Functional/ValueConversionType/OneToOneCompositeIdForeignKeyTest.php',
        'ORM/Functional/ValueConversionType/OneToOneCompositeIdTest.php',
        'ORM/Functional/ValueConversionType/OneToOneTest.php',
        'ORM/Functional/Ticket/DDC1452Test.php',
        'ORM/Functional/Ticket/DDC1113Test.php',
        'ORM/Functional/Ticket/DDC1181Test.php',
        'ORM/Functional/Ticket/DDC1193Test.php',
        'ORM/Functional/Ticket/DDC1225Test.php',
        'ORM/Functional/Ticket/DDC1228Test.php',
        'ORM/Functional/Ticket/DDC1238Test.php',
        'Doctrine\\Tests\\Models\\Quote\\NumericEntity',
        'ORM/Tools/SchemaValidatorTest.php',
        'ORM/Functional/SecondLevelCacheFunctionalTestCase.php',
        'ORM/Functional/SecondLevelCacheTest.php',
        'ORM/Functional/SecondLevelCacheCompositePrimaryKeyTest.php',
        'ORM/Functional/SecondLevelCacheCompositePrimaryKeyWithAssociationsTest.php',
        'ORM/Functional/SecondLevelCacheConcurrentTest.php',
        'ORM/Functional/SecondLevelCacheCountQueriesTest.php',
        'ORM/Functional/SecondLevelCacheCriteriaTest.php',
        'ORM/Functional/SecondLevelCacheExtraLazyCollectionTest.php',
        'ORM/Functional/SecondLevelCacheJoinTableInheritanceTest.php',
        'ORM/Functional/SecondLevelCacheManyToManyTest.php',
        'ORM/Functional/SecondLevelCacheManyToOneTest.php',
        'ORM/Functional/SecondLevelCacheOneToManyTest.php',
        'ORM/Functional/SecondLevelCacheOneToOneTest.php',
        'ORM/Functional/SecondLevelCacheQueryCacheTest.php',
        'ORM/Functional/SecondLevelCacheRepositoryTest.php',
        'ORM/Functional/SecondLevelCacheSingleTableInheritanceTest.php',
        'ORM/Functional/Ticket/DDC1250Test.php',
        'ORM/Functional/Ticket/DDC1300Test.php',
        'ORM/Functional/Ticket/DDC1335Test.php',
        'ORM/Functional/Ticket/DDC1400Test.php',
        'ORM/Functional/Ticket/DDC1430Test.php',
        'ORM/Functional/Ticket/DDC1436Test.php',
        'ORM/Functional/Ticket/DDC144Test.php',
        'ORM/Functional/Ticket/DDC1454Test.php',
        'ORM/Functional/Ticket/DDC1458Test.php',
        'ORM/Functional/Ticket/DDC1461Test.php',
        'ORM/Functional/Ticket/DDC1514Test.php',
        'ORM/Functional/Ticket/DDC1515Test.php',
        'ORM/Functional/Ticket/DDC1526Test.php',
        'ORM/Functional/Ticket/DDC1548Test.php',
        'ORM/Functional/Ticket/DDC1595Test.php',
        'ORM/Functional/Ticket/DDC1654Test.php',
        'ORM/Functional/Ticket/DDC1707Test.php',
        'ORM/Functional/Ticket/DDC1719Test.php',
        'ORM/Functional/Ticket/DDC1757Test.php',
        'ORM/Functional/Ticket/DDC1787Test.php',
        'ORM/Functional/Ticket/DDC1925Test.php',
        'ORM/Functional/Ticket/DDC192Test.php',
        'ORM/Functional/Ticket/DDC199Test.php',
        'ORM/Functional/Ticket/DDC2084Test.php',
        'ORM/Functional/Ticket/DDC2106Test.php',
        'ORM/Functional/Ticket/DDC211Test.php',
        'ORM/Functional/Ticket/DDC2175Test.php',
        'ORM/Functional/Ticket/DDC2214Test.php',
        'ORM/Functional/Ticket/DDC237Test.php',
        'ORM/Functional/Ticket/DDC2660Test.php',
        'ORM/Functional/Ticket/DDC2692Test.php',
        'ORM/Functional/Ticket/DDC2759Test.php',
        'ORM/Functional/Ticket/DDC2780Test.php',
        'ORM/Functional/Ticket/DDC279Test.php',
        'ORM/Functional/Ticket/DDC2895Test.php',
        'ORM/Functional/Ticket/DDC2931Test.php',
        'ORM/Functional/Ticket/DDC2996Test.php',
        'ORM/Functional/Ticket/DDC3042Test.php',
        'ORM/Functional/Ticket/DDC309Test.php',
        'ORM/Functional/Ticket/DDC3170Test.php',
        'ORM/Functional/Ticket/DDC3223Test.php',
        'ORM/Functional/Ticket/DDC3330Test.php',
        'ORM/Functional/Ticket/DDC345Test.php',
        'ORM/Functional/Ticket/DDC353Test.php',
        'ORM/Functional/Ticket/DDC3644Test.php',
        'ORM/Functional/Ticket/DDC371Test.php',
        'ORM/Functional/Ticket/DDC422Test.php',
        'ORM/Functional/Ticket/DDC425Test.php',
        'ORM/Functional/Ticket/DDC440Test.php',
        'ORM/Functional/Ticket/DDC444Test.php',
        'ORM/Functional/Ticket/DDC448Test.php',
        'ORM/Functional/Ticket/DDC493Test.php',
        'ORM/Functional/Ticket/DDC512Test.php',
        'ORM/Functional/Ticket/DDC513Test.php',
        'ORM/Functional/Ticket/DDC522Test.php',
        'ORM/Functional/Ticket/DDC531Test.php',
        'ORM/Functional/Ticket/DDC588Test.php',
        'ORM/Functional/Ticket/DDC599Test.php',
        'ORM/Functional/Ticket/DDC618Test.php',
        'ORM/Functional/Ticket/DDC633Test.php',
        'ORM/Functional/Ticket/DDC6558Test.php',
        'ORM/Functional/Ticket/DDC656Test.php',
        'ORM/Functional/Ticket/DDC6573Test.php',
        'ORM/Functional/Ticket/DDC698Test.php',
        'ORM/Functional/Ticket/DDC69Test.php',
        'ORM/Functional/Ticket/DDC719Test.php',
        'ORM/Functional/Ticket/DDC735Test.php',
        'ORM/Functional/Ticket/DDC809Test.php',
        'ORM/Functional/Ticket/DDC837Test.php',
        'ORM/Functional/Ticket/DDC881Test.php',
        'ORM/Functional/Ticket/DDC960Test.php',
        'ORM/Functional/Ticket/DDC992Test.php',
        'ORM/Functional/Ticket/DDC1080Test.php',
        'ORM/Functional/Ticket/DDC1163Test.php',
        'ORM/Functional/Ticket/DDC1209Test.php',
        'ORM/Functional/Ticket/DDC1685Test.php',
        'ORM/Functional/Ticket/DDC1695Test.php',
        'ORM/Functional/Ticket/DDC1884Test.php',
        'ORM/Functional/Ticket/DDC1995Test.php',
        'ORM/Functional/Ticket/DDC1998Test.php',
        'ORM/Functional/Ticket/DDC2012Test.php',
        'ORM/Functional/Ticket/DDC2090Test.php',
        'ORM/Functional/Ticket/DDC2138Test.php',
        'ORM/Functional/Ticket/DDC2182Test.php',
        'ORM/Functional/Ticket/DDC2224Test.php',
        'ORM/Functional/Ticket/DDC2252Test.php',
        'ORM/Functional/Ticket/DDC2306Test.php',
        'ORM/Functional/Ticket/DDC2346Test.php',
        'ORM/Functional/Ticket/DDC2350Test.php',
        'ORM/Functional/Ticket/DDC2359Test.php',
        'ORM/Functional/Ticket/DDC2387Test.php',
        'ORM/Functional/Ticket/DDC2415Test.php',
        'ORM/Functional/Ticket/DDC2494Test.php',
        'ORM/Functional/Ticket/DDC2575Test.php',
        'ORM/Functional/Ticket/DDC2579Test.php',
        'ORM/Functional/Ticket/DDC258Test.php',
        'ORM/Functional/Ticket/DDC2602Test.php',
        'ORM/Functional/Ticket/DDC2775Test.php',
        'ORM/Functional/Ticket/DDC2825Test.php',
        'ORM/Functional/Ticket/DDC2862Test.php',
        'ORM/Functional/Ticket/DDC2943Test.php',
        'ORM/Functional/Ticket/DDC2984Test.php',
        'ORM/Functional/Ticket/DDC3033Test.php',
        'ORM/Functional/Ticket/DDC3068Test.php',
        'ORM/Functional/Ticket/DDC3103Test.php',
        'ORM/Functional/Ticket/DDC3192Test.php',
        'ORM/Functional/Ticket/DDC3300Test.php',
        'ORM/Functional/Ticket/DDC3303Test.php',
        'ORM/Functional/Ticket/DDC3346Test.php',
        'ORM/Functional/Ticket/DDC3582Test.php',
        'ORM/Functional/Ticket/DDC3597Test.php',
        'ORM/Functional/Ticket/DDC3634Test.php',
        'ORM/Functional/Ticket/DDC3785Test.php',
        'ORM/Functional/Ticket/DDC381Test.php',
        'ORM/Functional/Ticket/DDC3967Test.php',
        'ORM/Functional/Ticket/DDC4003Test.php',
        'ORM/Functional/Ticket/DDC5684Test.php',
        'ORM/Functional/Ticket/DDC6460Test.php',
        'ORM/Functional/Ticket/DDC657Test.php',
        'ORM/Functional/Ticket/DDC7969Test.php',
        'ORM/Functional/Ticket/DDC933Test.php',
        'ORM/Functional/Ticket/EagerSecondLevelCacheCollectionTest.php',
        'ORM/Functional/Ticket/GH10049/GH10049Test.php',
        'ORM/Functional/Ticket/GH10132Test.php',
        'ORM/Functional/Ticket/GH10288Test.php',
        'ORM/Functional/Ticket/GH10334Test.php',
        'ORM/Functional/Ticket/GH10336Test.php',
        'ORM/Functional/Ticket/GH10348Test.php',
        'ORM/Functional/Ticket/GH10387Test.php',
        'ORM/Functional/Ticket/GH10450Test.php',
        'ORM/Functional/Ticket/GH10454Test.php',
        'ORM/Functional/Ticket/GH10462Test.php',
        'ORM/Functional/Ticket/GH10473Test.php',
        'ORM/Functional/Ticket/GH10531Test.php',
        'ORM/Functional/Ticket/GH10532Test.php',
        'ORM/Functional/Ticket/GH10566Test.php',
        'ORM/Functional/Ticket/GH10625Test.php',
        'ORM/Functional/Ticket/GH10661/GH10661Test.php',
        'ORM/Functional/Ticket/GH10747Test.php',
        'ORM/Functional/Ticket/GH10752Test.php',
        'ORM/Functional/Ticket/GH10808Test.php',
        'ORM/Functional/Ticket/GH10869Test.php',
        'ORM/Functional/Ticket/GH10880Test.php',
        'ORM/Functional/Ticket/GH10889Test.php',
        'ORM/Functional/Ticket/GH10912Test.php',
        'ORM/Functional/Ticket/GH10913Test.php',
        'ORM/Functional/Ticket/GH10927Test.php',
        'ORM/Functional/Ticket/GH11017/GH11017Test.php',
        'ORM/Functional/Ticket/GH11037/GH11037Test.php',
        'ORM/Functional/Ticket/GH11058Test.php',
        'ORM/Functional/Ticket/GH11072/GH11072Test.php',
        'ORM/Functional/Ticket/GH11112Test.php',
        'ORM/Functional/Ticket/GH11135Test.php',
        'ORM/Functional/Ticket/GH11149/GH11149Test.php',
        'ORM/Functional/Ticket/GH11163Test.php',
        'ORM/Functional/Ticket/GH11199Test.php',
        'ORM/Functional/Ticket/GH11341Test.php',
        'ORM/Functional/Ticket/GH11386/GH11386Test.php',
        'ORM/Functional/Ticket/GH11487Test.php',
        'ORM/Functional/Ticket/GH11524Test.php',
        'ORM/Functional/Ticket/GH11982Test.php',
        'ORM/Functional/Ticket/GH12166/GH12166Test.php',
        'ORM/Functional/Ticket/GH12172Test.php',
        'ORM/Functional/Ticket/GH12174Test.php',
        'ORM/Functional/Ticket/GH12183Test.php',
        'ORM/Functional/Ticket/GH12225/GH12225Test.php',
        'ORM/Functional/Ticket/GH12254Test.php',
        'ORM/Functional/Ticket/GH12403Test.php',
        'ORM/Functional/Ticket/GH12438Test.php',
        'ORM/Functional/Ticket/GH12505/GH12505Test.php',
        'ORM/Functional/Ticket/GH2947Test.php',
        'ORM/Functional/Ticket/GH5562Test.php',
        'ORM/Functional/Ticket/GH5742Test.php',
        'ORM/Functional/Ticket/GH5762Test.php',
        'ORM/Functional/Ticket/GH5804Test.php',
        'ORM/Functional/Ticket/GH5887Test.php',
        'ORM/Functional/Ticket/GH5988Test.php',
        'ORM/Functional/Ticket/GH5998Test.php',
        'ORM/Functional/Ticket/GH6029Test.php',
        'ORM/Functional/Ticket/GH6123Test.php',
        'ORM/Functional/Ticket/GH6141Test.php',
        'ORM/Functional/Ticket/GH6217Test.php',
        'ORM/Functional/Ticket/GH6362Test.php',
        'ORM/Functional/Ticket/GH6394Test.php',
        'ORM/Functional/Ticket/GH6402Test.php',
        'ORM/Functional/Ticket/GH6464Test.php',
        'ORM/Functional/Ticket/GH6499OneToManyRelationshipTest.php',
        'ORM/Functional/Ticket/GH6499OneToOneRelationshipTest.php',
        'ORM/Functional/Ticket/GH6499Test.php',
        'ORM/Functional/Ticket/GH6531Test.php',
        'ORM/Functional/Ticket/GH6682Test.php',
        'ORM/Functional/Ticket/GH6699Test.php',
        'ORM/Functional/Ticket/GH6740Test.php',
        'ORM/Functional/Ticket/GH6823Test.php',
        'ORM/Functional/Ticket/GH6937Test.php',
        'ORM/Functional/Ticket/GH7006Test.php',
        'ORM/Functional/Ticket/GH7012Test.php',
        'ORM/Functional/Ticket/GH7062Test.php',
        'ORM/Functional/Ticket/GH7067Test.php',
        'ORM/Functional/Ticket/GH7068Test.php',
        'ORM/Functional/Ticket/GH7079Test.php',
        'ORM/Functional/Ticket/GH7180Test.php',
        'ORM/Functional/Ticket/GH7259Test.php',
        'ORM/Functional/Ticket/GH7286Test.php',
        'ORM/Functional/Ticket/GH7366Test.php',
        'ORM/Functional/Ticket/GH7496WithToIterableTest.php',
        'ORM/Functional/Ticket/GH7505Test.php',
        'ORM/Functional/Ticket/GH7512Test.php',
        'ORM/Functional/Ticket/GH7629Test.php',
        'ORM/Functional/Ticket/GH7661Test.php',
        'ORM/Functional/Ticket/GH7684Test.php',
        'ORM/Functional/Ticket/GH7717Test.php',
        'ORM/Functional/Ticket/GH7735Test.php',
        'ORM/Functional/Ticket/GH7737Test.php',
        'ORM/Functional/Ticket/GH7761Test.php',
        'ORM/Functional/Ticket/GH7767Test.php',
        'ORM/Functional/Ticket/GH7820Test.php',
        'ORM/Functional/Ticket/GH7829Test.php',
        'ORM/Functional/Ticket/GH7836Test.php',
        'ORM/Functional/Ticket/GH7864Test.php',
        'ORM/Functional/Ticket/GH7869Test.php',
        'ORM/Functional/Ticket/GH7875Test.php',
        'ORM/Functional/Ticket/GH7941Test.php',
        'ORM/Functional/Ticket/GH8055Test.php',
        'ORM/Functional/Ticket/GH8061Test.php',
        'ORM/Functional/Ticket/GH8127Test.php',
        'ORM/Functional/Ticket/GH8217Test.php',
        'ORM/Functional/Ticket/GH8415Test.php',
        'ORM/Functional/Ticket/GH8415ToManyAssociationTest.php',
        'ORM/Functional/Ticket/GH8443Test.php',
        'ORM/Functional/Ticket/GH8499Test.php',
        'ORM/Functional/Ticket/GH8663Test.php',
        'ORM/Functional/Ticket/GH8914Test.php',
        'ORM/Functional/Ticket/GH9027Test.php',
        'ORM/Functional/Ticket/GH9109Test.php',
        'ORM/Functional/Ticket/GH9192Test.php',
        'ORM/Functional/Ticket/GH9230Test.php',
        'ORM/Functional/Ticket/GH9335Test.php',
        'ORM/Functional/Ticket/GH9467/GH9467Test.php',
        'ORM/Functional/Ticket/GH9516Test.php',
        'ORM/Functional/Ticket/GH9538Test.php',
        'ORM/Functional/Ticket/GH9579Test.php',
        'ORM/Functional/Ticket/GH9807Test.php',
        'ORM/Functional/Ticket/Issue5989Test.php',
        'ORM/Functional/Ticket/Issue9300Test.php',
        'ORM/Functional/Ticket/LazyEagerCollectionTest.php',
        'ORM/Functional/Ticket/SwitchContextWithFilter/SwitchContextTest.php',
        'ORM/Functional/Ticket/Ticket2481Test.php',
        'ORM/Functional/Ticket/Ticket4646InstanceOfAbstractTest.php',
        'ORM/Functional/Ticket/Ticket4646InstanceOfMultiLevelTest.php',
        'ORM/Functional/Ticket/Ticket4646InstanceOfParametricTest.php',
        'ORM/Functional/Ticket/Ticket4646InstanceOfTest.php',
        'ORM/Functional/Ticket/Ticket4646InstanceOfWithMultipleParametersTest.php',
        'ORM/Functional/Ticket/GH10049/ReadOnlyPropertyInheritor.php',
        'ORM/Functional/Ticket/GH10049/ReadOnlyPropertyOwner.php',
        'ORM/Functional/Ticket/GH10661/InvalidChildEntity.php',
        'ORM/Functional/Ticket/GH10661/InvalidEntity.php',
        'ORM/Functional/Ticket/GH11017/GH11017Entity.php',
        'ORM/Functional/Ticket/GH11017/GH11017Enum.php',
        'ORM/Functional/Ticket/GH11037/EntityStatus.php',
        'ORM/Functional/Ticket/GH11037/IntEntityStatus.php',
        'ORM/Functional/Ticket/GH11037/InvalidEntityWithTypedEnum.php',
        'ORM/Functional/Ticket/GH11037/StringEntityStatus.php',
        'ORM/Functional/Ticket/GH11037/ValidEntityWithTypedEnum.php',
        'ORM/Functional/Ticket/GH11072/GH11072EntityAdvanced.php',
        'ORM/Functional/Ticket/GH11072/GH11072EntityBasic.php',
        'ORM/Functional/Ticket/GH11149/EagerProduct.php',
        'ORM/Functional/Ticket/GH11149/EagerProductTranslation.php',
        'ORM/Functional/Ticket/GH11149/Locale.php',
        'ORM/Functional/Ticket/GH11386/GH11386EntityCart.php',
        'ORM/Functional/Ticket/GH11386/GH11386EntityCustomer.php',
        'ORM/Functional/Ticket/GH11386/GH11386EnumType.php',
        'ORM/Functional/Ticket/GH12166/LazyEntityWithReadonlyId.php',
        'ORM/Functional/Ticket/GH12225/AbstractDirectory.php',
        'ORM/Functional/Ticket/GH12225/ConcreteDirectory.php',
        'ORM/Functional/Ticket/GH12225/MyFilter.php',
        'ORM/Functional/Ticket/GH12505/Car.php',
        'ORM/Functional/Ticket/GH12505/Garage.php',
        'ORM/Functional/Ticket/GH12505/SoftDeleteFilter.php',
        'ORM/Functional/Ticket/GH12505/Vehicle.php',
        'ORM/Functional/Ticket/GH9467/JoinedInheritanceChild.php',
        'ORM/Functional/Ticket/GH9467/JoinedInheritanceNonInsertableColumn.php',
        'ORM/Functional/Ticket/GH9467/JoinedInheritanceNonUpdatableColumn.php',
        'ORM/Functional/Ticket/GH9467/JoinedInheritanceNonWritableColumn.php',
        'ORM/Functional/Ticket/GH9467/JoinedInheritanceRoot.php',
        'ORM/Functional/Ticket/GH9467/JoinedInheritanceWritableColumn.php',
        'ORM/Functional/EagerFetchCollectionTest.php',
        'ORM/Tools/ResolveTargetEntityListenerTest.php',
        'ORM/Tools/AttachEntityListenersListenerTest.php',
        'ORM/Tools/Console/MetadataFilterTest.php',
        'ORM/Entity/ConstructorTest.php',
        'ORM/Functional/AdvancedAssociationTest.php',
        'ORM/Functional/CascadeRemoveOrderTest.php',
        'ORM/Functional/DefaultValuesTest.php',
        'ORM/Functional/GH7877Test.php',
        'ORM/Functional/InvalidMappingDefinitionTest.php',
        'ORM/Functional/PrePersistEventTest.php',
        'ORM/Functional/SequenceGeneratorTest.php',
        'Models/VersionedOneToOne/FirstRelatedEntity.php',
        'Models/VersionedOneToOne/SecondRelatedEntity.php',
        'Models/AbstractFetchEager/AbstractRemoteControl.php',
        'Models/AbstractFetchEager/MobileRemoteControl.php',
        'Models/AbstractFetchEager/User.php',
        'Models/EagerFetchedCompositeOneToMany/RootEntity.php',
        'Models/EagerFetchedCompositeOneToMany/SecondLevel.php',
        'Models/EagerFetchedCompositeOneToMany/SecondLevelWithoutCompositePrimaryKey.php',
        'ORM/Tools/Console/Command/SchemaTool/Models/Keyboard.php',
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

    // The model sets of SchemaValidatorTest::modelSetProvider().
    foreach (['CMS', 'Company', 'ECommerce', 'Forum', 'Navigation', 'Routing'] as $modelSet) {
        foreach (glob("$root/orm/tests/Tests/Models/$modelSet/*.php") as $modelFile) {
            require_once $modelFile;
        }
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
