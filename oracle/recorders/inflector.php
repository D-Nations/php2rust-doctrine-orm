<?php
// Records doctrine/inflector for the Rust port:
// crates/php2rust-doctrine-inflector/tests/data/inflector.json.
//
//   scripts/record.sh inflector
//
// "providers": the upstream tests' data providers, run (PHPUnit's TestCase
// is a stub here; no test runs). "words": singularize() and pluralize() of
// every provider word, as given, ucfirst() and strtoupper(), in all eight
// languages. "strings": tableize(), classify(), camelize(), capitalize()
// (default and '-_ ' delimiters), seemsUtf8(), unaccent() and urlize() of
// the providers' strings, hand-picked ones (accents, Greek sigma, Turkish
// I, ISO-8859-1 bytes, invalid UTF-8) and 400 drawn with mt_rand(); an
// exception is [class, message]. Strings are hex.

declare(strict_types=1);

namespace PHPUnit\Framework {
    abstract class TestCase
    {
    }
}

namespace {
    use Doctrine\Inflector\InflectorFactory;
    use Doctrine\Inflector\Language;

    $root = dirname(__DIR__, 2) . '/upstream/doctrine/inflector';
    spl_autoload_register(static function (string $class) use ($root): void {
        foreach (['Doctrine\\Tests\\Inflector\\' => "$root/tests/", 'Doctrine\\Inflector\\' => "$root/src/"] as $prefix => $dir) {
            if (str_starts_with($class, $prefix)) {
                $file = $dir . str_replace('\\', '/', substr($class, strlen($prefix))) . '.php';
                if (is_file($file)) {
                    require $file;
                }

                return;
            }
        }
    });

    $tests = 'Doctrine\\Tests\\Inflector\\';
    $languages = [
        'English' => Language::ENGLISH,
        'Esperanto' => Language::ESPERANTO,
        'French' => Language::FRENCH,
        'Italian' => Language::ITALIAN,
        'NorwegianBokmal' => Language::NORWEGIAN_BOKMAL,
        'Portuguese' => Language::PORTUGUESE,
        'Spanish' => Language::SPANISH,
        'Turkish' => Language::TURKISH,
    ];

    $hex = static fn (array $rows): array => array_map(
        static fn (array $row): array => array_map('bin2hex', $row),
        array_values($rows),
    );
    $providers = [
        'InflectorTest::dataStringsUrlize' => $hex(($tests . 'InflectorTest')::dataStringsUrlize()),
        'InflectorFunctionalTest::dataStringsTableize' => $hex(($tests . 'InflectorFunctionalTest')::dataStringsTableize()),
        'InflectorFunctionalTest::dataStringsClassify' => $hex(($tests . 'InflectorFunctionalTest')::dataStringsClassify()),
        'InflectorFunctionalTest::dataStringsCamelize' => $hex(($tests . 'InflectorFunctionalTest')::dataStringsCamelize()),
        'EnglishFunctionalTest::dataSingularsUninflectedWhenSingularized' => $hex(($tests . 'Rules\\English\\EnglishFunctionalTest')::dataSingularsUninflectedWhenSingularized()),
        'EnglishFunctionalTest::dataPluralUninflectedWhenPluralized' => $hex(($tests . 'Rules\\English\\EnglishFunctionalTest')::dataPluralUninflectedWhenPluralized()),
    ];
    $words = [];
    foreach (array_keys($languages) as $name) {
        $class = "{$tests}Rules\\$name\\{$name}FunctionalTest";
        $rows = $class::dataSampleWords();
        $providers["{$name}FunctionalTest::dataSampleWords"] = $hex($rows);
        foreach ($rows as $row) {
            array_push($words, ...$row);
        }
    }
    foreach ($providers['EnglishFunctionalTest::dataSingularsUninflectedWhenSingularized'] as $row) {
        $words[] = hex2bin($row[0]);
    }
    foreach ($providers['EnglishFunctionalTest::dataPluralUninflectedWhenPluralized'] as $row) {
        $words[] = hex2bin($row[0]);
    }
    $variants = [];
    foreach (array_unique($words) as $word) {
        array_push($variants, $word, ucfirst($word), strtoupper($word));
    }
    $variants = array_values(array_unique($variants));

    $inflected = [];
    foreach ($languages as $name => $language) {
        $inflector = InflectorFactory::createForLanguage($language)->build();
        $inflected[$name] = [
            'singularize' => array_map(static fn ($w) => bin2hex($inflector->singularize($w)), $variants),
            'pluralize' => array_map(static fn ($w) => bin2hex($inflector->pluralize($w)), $variants),
        ];
    }

    $strings = [
        '', 'ModelName', 'modelName', 'model_name', 'HTMLParser', 'XMLHttpRequest', 'F0oBar', 'a1B2c3D',
        'foo bar', 'foo  bar', 'foo_bar_', '_foo', '-foo-', 'foo--bar--', '--x--', 'a_b-c d',
        "tab\tsep\nnl", "x\0y\x0Bz", 'top-o-the-morning to all_of_you!', 'Testing_Creating a -Slug!@#',
        'test1::test2', 'test1$test2', 'TESTAb-test2', 'a/b/c', '100% sure', 'C++ & C#',
        "\xC3\x80\xC3\x89\xC3\x8E\xC3\xB5\xC3\xBC", "\xC3\x89coleNormale", "\xC4\xB0stanbul", "I\xCC\x87",
        "\xCE\xA3\xCE\x91\xCE\xA3", "\xCE\x9F\xCE\x94\xCE\x9F\xCE\xA3 \xCE\xA3", "stra\xC3\x9Fe", "\xC3\xBCber_cool",
        "\xC7\x85ungla", "\xEF\xAC\x81le", "\xC2\xA3100 \xE2\x82\xAC5", "\xE2\x85\xAB", "\xC5\x92uvre \xC5\x93uf",
        "\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E", "\xF0\x9F\x98\x80 smile", "\xE9t\xE9", "\xC3", "a\x80b",
        "\x8C\x9C\xC6\xDE\xDF", "caf\xE9 cr\xE8me", "\xC0\xC1\xFF", "\xF8\x88\x80\x80\x80", "\xED\xA0\x80",
        "\xE2\x82", "Ab\xC3\x9FCd", "D\xC3\xA9j\xC3\xA0Vu", "mixed\xC3\xA9\xE9",
    ];
    foreach ($providers as $rows) {
        foreach ($rows as $row) {
            foreach ($row as $cell) {
                $strings[] = hex2bin($cell);
            }
        }
    }
    mt_srand(2026);
    $alphabet = [
        'a', 'B', 'c', 'D', 'x', 'Z', '0', '7', ' ', '_', '-', '.', ':', '/', '$', "\t", '!', "\xC3\xA9",
        "\xC3\x84", "\xC3\x9F", "\xC4\xB0", "\xC4\xB1", "\xCE\xA3", "\xCF\x83", "\xC5\x93", "\xE2\x82\xAC",
        "\xC2\xA3", "\xE6\x97\xA5", "\xE9", "\xC3", "\x80",
    ];
    for ($i = 0; $i < 400; $i++) {
        $length = mt_rand(0, 12);
        $s = '';
        for ($j = 0; $j < $length; $j++) {
            $s .= $alphabet[mt_rand(0, count($alphabet) - 1)];
        }
        $strings[] = $s;
    }
    $strings = array_values(array_unique($strings));

    $inflector = InflectorFactory::create()->build();
    $call = static function (callable $f): array|string {
        try {
            return bin2hex((string) $f());
        } catch (Throwable $e) {
            return [get_class($e), bin2hex($e->getMessage())];
        }
    };
    $stringResults = [];
    foreach ($strings as $s) {
        $stringResults[] = [
            'input' => bin2hex($s),
            'tableize' => $call(static fn () => $inflector->tableize($s)),
            'classify' => $call(static fn () => $inflector->classify($s)),
            'camelize' => $call(static fn () => $inflector->camelize($s)),
            'capitalize' => $call(static fn () => $inflector->capitalize($s)),
            'capitalize_custom' => $call(static fn () => $inflector->capitalize($s, '-_ ')),
            'seems_utf8' => $inflector->seemsUtf8($s),
            'unaccent' => $call(static fn () => $inflector->unaccent($s)),
            'urlize' => $call(static fn () => $inflector->urlize($s)),
        ];
    }

    echo json_encode([
        'php' => PHP_VERSION,
        'providers' => $providers,
        'words' => array_map('bin2hex', $variants),
        'inflected' => $inflected,
        'strings' => $stringResults,
    ], JSON_THROW_ON_ERROR), "\n";
}
