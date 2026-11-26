<?php
// Records how doctrine/lexer's AbstractLexer splits inputs (its one
// preg_split() call and the tokens built from it) for the Rust port:
// crates/php2rust-doctrine-lexer/tests/data/lexer.json.
//
//   scripts/record.sh lexer
//
// Strings are hex (inputs may be invalid UTF-8). Each lexer is given its
// patterns and modifiers; "raw" lexers leave values alone and give no type,
// "concrete" is the tests' ConcreteLexer. PHP's warnings (an invalid
// pattern) are recorded, not printed.

declare(strict_types=1);

$root = dirname(__DIR__, 2) . '/upstream/doctrine/lexer';
require "$root/src/Token.php";
require "$root/src/AbstractLexer.php";
require "$root/tests/ConcreteLexer.php";

use Doctrine\Common\Lexer\AbstractLexer;
use Doctrine\Tests\Common\Lexer\ConcreteLexer;

final class RawLexer extends AbstractLexer
{
    /**
     * @param list<string> $catchable
     * @param list<string> $nonCatchable
     */
    public function __construct(
        private array $catchable,
        private array $nonCatchable,
        private string $modifiers,
    ) {
    }

    protected function getCatchablePatterns(): array
    {
        return $this->catchable;
    }

    protected function getNonCatchablePatterns(): array
    {
        return $this->nonCatchable;
    }

    protected function getModifiers(): string
    {
        return $this->modifiers;
    }

    protected function getType(string &$value): string|null
    {
        return null;
    }
}

$concrete = [['=|<|>', '[a-z]+', '\d+'], ['\s+', '(.)'], 'iu'];
$lexers = [
    'concrete' => $concrete,
    'concrete_raw' => $concrete,
    'concrete_ascii' => [$concrete[0], $concrete[1], 'i'],
    'dql' => [
        [
            '[a-z_][a-z0-9_]*\:[a-z_][a-z0-9_]*(?:\\\[a-z_][a-z0-9_]*)*',
            '[a-z_\\\][a-z0-9_]*(?:\\\[a-z_][a-z0-9_]*)*',
            '(?:[0-9]+(?:[\.][0-9]+)*)(?:e[+-]?[0-9]+)?',
            "'(?:[^']|'')*'",
            '\?[0-9]*|:[a-z_][a-z0-9_]*',
        ],
        ['\s+', '--.*', '(.)'],
        'iu',
    ],
    'annotation' => [
        [
            '[a-z_\\\][a-z0-9_\:\\\]*[a-z_][a-z0-9_]*',
            '(?:[+-]?[0-9]+(?:[\.][0-9]+)*)(?:[eE][+-]?[0-9]+)?',
            '"(?:""|[^"])*+"',
        ],
        ['[\s,]+'],
        'iu',
    ],
    'single_letter' => [['[a-z]'], ['[\s,]+'], 'iu'],
    'case_sensitive' => [['[a-z]+', '[0-9]+'], ['\s+'], 'u'],
    'empty_matches' => [['x*'], ['\s+'], 'iu'],
    'empty_and_groups' => [['a', 'b*'], ['(c)(d)?', '\s+'], 'iu'],
    'lazy' => [['a*?', 'b'], ['\s+'], 'iu'],
    'extended' => [["[a-z]+ # word\n", "\\d* # digits\n"], ['\s+'], 'iux'],
    'extended_open_comment' => [['[a-z]+ # word', '\d* # digits'], ['\s+'], 'iux'],
    'backreference' => [['(a)\2', '[b-z]+'], ['\s+'], 'iu'],
    'invalid' => [['('], ['\s+'], 'iu'],
];

$fixed = [
    '', ' ', 'price=10', 'price = 10', 'a b  c', "\n\t x \r\n", 'one', 'ONE two',
    "\xE9=10", "caf\xC3\xA9 = 12", "h\xC3\xA9llo w\xC3\xB6rld 12", "\xE2\x84\xAA", "\xC5\xBF",
    "\xC4\xB0\xC4\xB1", "\xE6\x97\xA5\xE6\x9C\xAC 123", '1.5e3', '1.2.3', 'a,b,,c', 'x', 'xx a xxx',
    'abcd', 'cd c', 'bbb ccc d',
    "SELECT u FROM App\\Entity\\User u WHERE u.id = :id AND u.name = 'x''y' -- end",
    'SELECT a:b FROM c WHERE d = ?1 OR e = ?', '@ODM\\Id', '@Foo("bar", 1.5e-3)', "'open",
    "\x00", "a\x00b", "\xFF\xFE",
];

mt_srand(20261126);
$alphabet = [
    'a', 'b', 'c', 'd', 'Z', 'x', 'y', 'e', '1', '0', '9', '.', '=', '<', '>', ' ', "\t", "\n", ',',
    ':', '?', "'", '"', '\\', '-', '_', '(', ')', '@', '+', "\xC3\xA9", "\xC4\xB0", "\xC4\xB1",
    "\xE2\x84\xAA", "\xC5\xBF", "\xE6\x97\xA5", "\xE9", "\xC3",
];
$inputs = $fixed;
for ($i = 0; $i < 400; $i++) {
    $length = mt_rand(0, 14);
    $input = '';
    for ($j = 0; $j < $length; $j++) {
        $input .= $alphabet[mt_rand(0, count($alphabet) - 1)];
    }
    $inputs[] = $input;
}

$warnings = [];
set_error_handler(static function (int $errno, string $message) use (&$warnings): bool {
    $warnings[] = $message;

    return true;
});

function encode(mixed $value): array
{
    return match (true) {
        is_string($value) => ['s' => bin2hex($value)],
        is_int($value) => ['i' => $value],
        default => throw new LogicException(get_debug_type($value)),
    };
}

$out = [];
foreach ($lexers as $name => [$catchable, $nonCatchable, $modifiers]) {
    $lexer = $name === 'concrete' ? new ConcreteLexer() : new RawLexer($catchable, $nonCatchable, $modifiers);
    $cases = [];
    foreach ($inputs as $input) {
        $warnings = [];
        $lexer->setInput($input);
        $tokens = [];
        while (($token = $lexer->peek()) !== null) {
            $tokens[] = [encode($token->value), $token->type, $token->position];
        }
        $cases[] = ['input' => bin2hex($input), 'tokens' => $tokens, 'warnings' => $warnings];
    }
    $out[] = [
        'name' => $name,
        'catchable' => array_map('bin2hex', $catchable),
        'non_catchable' => array_map('bin2hex', $nonCatchable),
        'modifiers' => $modifiers,
        'cases' => $cases,
    ];
}

echo json_encode(['php' => PHP_VERSION, 'lexers' => $out], JSON_THROW_ON_ERROR), "\n";
